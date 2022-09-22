use futures::FutureExt;
use tokio::{
    spawn,
    sync::{mpsc::Receiver, oneshot::Sender},
};

use attest_util::INFER_UNIT;
use tracing::{debug, info};

use crate::attestations::client::{AttestationClient, ServiceUrl};

use super::*;

#[derive(Hash, Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Serialize, Deserialize, Debug)]
pub enum PeerType {
    Push,
    Fetch,
}

#[derive(Hash, Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct Global;
pub type TaskType = PeerType;

pub type TaskID = (ServiceUrl, TaskType, bool);

pub enum PeerQuery {
    RunningTasks(Sender<Vec<TaskID>>),
    RefreshTasks,
}
pub fn startup(
    g: Arc<Globals>,
    db: MsgDB,
    mut status: Receiver<PeerQuery>,
) -> JoinHandle<Result<(), Box<dyn Error + Sync + Send + 'static>>> {
    tokio::spawn(async move {
        info!("Starting Task for Peer Services");
        let client = g.get_client().await?;
        let mut interval = g.config.peer_service.timer_override.reconnect_interval();
        let mut task_set: HashMap<TaskID, JoinHandle<Result<(), _>>> = HashMap::new();
        let _tip_attacher = spawn({
            let db = db.clone();
            let mut interval = g
                .config
                .peer_service
                .timer_override
                .attach_tip_while_busy_interval();
            let g = g.clone();
            async move {
                while !g.shutdown.should_quit() {
                    interval.tick().await;
                    let handle = db.get_handle().await;
                    let n_attached = handle.attach_tips();
                    info!(?n_attached, "Attached Tips");
                }
            }
        });
        'outer: while !g.shutdown.should_quit() {
            tokio::select! {
                query = status.recv() => {
                    match query {
                        Some(query) => {
                            match query {
                                PeerQuery::RunningTasks(r) => {
                                    r.send(task_set.keys().cloned().collect()).ok();
                                },
                                PeerQuery::RefreshTasks => {
                                    info!("Refresh Tasks");
                                }
                            }
                        }
                        None => continue 'outer,
                    }
                }
                _ = interval.tick() => { // do main loop
                    if g.shutdown.should_quit() {
                        break 'outer;
                    }
                }
            };
            info!("Scanning for service reboot");
            let mut create_services: HashSet<_> = db
                .get_handle()
                .await
                .get_all_hidden_services()?
                .into_iter()
                .flat_map(|p| {
                    let mut v = [None, None];
                    let service = ServiceUrl(p.service_url.into(), p.port);
                    if p.fetch_from {
                        v[0] = Some((service.clone(), PeerType::Fetch, p.allow_unsolicited_tips))
                    }
                    if p.push_to {
                        v[1] = Some((service, PeerType::Push, p.allow_unsolicited_tips))
                    }
                    v
                })
                .flatten()
                .collect();
            // Drop anything that is finished, we will re-add it later if still in create_services
            task_set.retain(|k, v| {
                if !v.is_finished() {
                    true
                } else {
                    // this is safe because v is finished
                    let val = v.now_or_never();
                    info!(result=?val,"Task Finished: {:?}", k);
                    false
                }
            });
            // If it is no longer in our services DB, drop it / disconnect, and also
            // remove it from our to create set (only if it is in the to retain set, mind you)
            task_set.retain(
                |service_id, service| match create_services.take(service_id) {
                    Some(_) => true,
                    None => {
                        info!("Aborting Task: {:?}", service_id);
                        service.abort();
                        false
                    }
                },
            );
            // Open connections to all services on the list and put into our task set.
            for task_id in create_services.into_iter() {
                info!("Starting Task: {:?}", task_id);
                let client = client.clone();
                match task_id.1 {
                    PeerType::Push => {
                        task_set.insert(
                            task_id.clone(),
                            tokio::spawn({
                                let g = g.clone();
                                let db = db.clone();
                                async move {
                                    push_peer::push_to_peer(g, client, &task_id.0, db).await
                                }
                            }),
                        );
                    }
                    PeerType::Fetch => {
                        task_set.insert(
                            task_id.clone(),
                            tokio::spawn({
                                let g = g.clone();
                                let db = db.clone();
                                async move {
                                    fetch_peer::fetch_from_peer(
                                        g, client, &task_id.0, db, task_id.2,
                                    )
                                    .await
                                }
                            }),
                        );
                    }
                }
            }
        }
        INFER_UNIT
    })
}

mod push_peer;

mod fetch_peer;
