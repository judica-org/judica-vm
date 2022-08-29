use futures::FutureExt;
use tokio::{
    spawn,
    sync::{mpsc::Receiver, oneshot::Sender},
};

use attest_util::INFER_UNIT;
use tracing::{debug, info};

use crate::attestations::client::AttestationClient;

use super::*;

#[derive(Hash, Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Serialize, Deserialize, Debug)]
pub enum PeerType {
    Push,
    Fetch,
}

#[derive(Hash, Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct Global;
pub type TaskType = PeerType;

pub type TaskID = (String, u16, TaskType, bool);

pub enum PeerQuery {
    RunningTasks(Sender<Vec<TaskID>>),
}
pub fn startup(
    config: Arc<Config>,
    db: MsgDB,
    quit: AppShutdown,
    mut status: Receiver<PeerQuery>,
) -> JoinHandle<Result<(), Box<dyn Error + Sync + Send + 'static>>> {
    let jh = tokio::spawn(async move {
        let mut bld = reqwest::Client::builder();
        if let Some(tor_config) = config.tor.clone() {
            // Local Pass if in test mode
            // TODO: make this programmatic?
            #[cfg(test)]
            {
                bld = bld.proxy(reqwest::Proxy::custom(move |url| {
                    if url.host_str() == Some("127.0.0.1") {
                        Some("127.0.0.1")
                    } else {
                        None
                    }
                }));
            }
            let proxy =
                reqwest::Proxy::all(format!("socks5h://127.0.0.1:{}", tor_config.socks_port))?;
            bld = bld.proxy(proxy);
        }
        let inner_client = bld.build()?;
        let client = AttestationClient::new(inner_client);
        let secp = Arc::new(Secp256k1::new());
        let mut interval = config.peer_service.timer_override.reconnect_interval();
        let mut task_set: HashMap<TaskID, JoinHandle<Result<(), _>>> = HashMap::new();
        let _tip_attacher = spawn({
            let db = db.clone();
            let quit = quit.clone();
            let mut interval = config
                .peer_service
                .timer_override
                .attach_tip_while_busy_interval();
            async move {
                while !quit.should_quit() {
                    interval.tick().await;
                    let handle = db.get_handle().await;
                    let n_attached = handle.attach_tips();
                    info!(?n_attached, "Attached Tips");
                }
            }
        });
        'outer: while !quit.should_quit() {
            tokio::select! {
                query = status.recv() => {
                    match query {
                        Some(query) => {
                            match query {
                                PeerQuery::RunningTasks(r) => {
                                    r.send(task_set.keys().cloned().collect()).ok();
                                },
                            }
                        }
                        None => continue 'outer,
                    }
                }
                _ = interval.tick() => { // do main loop
                    if quit.should_quit() {
                        break 'outer;
                    }
                }
            };
            let mut create_services: HashSet<_> = db
                .get_handle()
                .await
                .get_all_hidden_services()?
                .into_iter()
                .flat_map(|p| {
                    let mut v = [None, None];
                    if p.fetch_from {
                        v[0] = Some((
                            p.service_url.clone(),
                            p.port,
                            PeerType::Fetch,
                            p.allow_unsolicited_tips,
                        ))
                    }
                    if p.push_to {
                        v[1] = Some((
                            p.service_url,
                            p.port,
                            PeerType::Push,
                            p.allow_unsolicited_tips,
                        ))
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
                match task_id.2 {
                    PeerType::Push => {
                        task_set.insert(
                            task_id.clone(),
                            tokio::spawn(push_peer::push_to_peer(
                                config.clone(),
                                secp.clone(),
                                client,
                                (task_id.0, task_id.1),
                                db.clone(),
                                quit.clone(),
                            )),
                        );
                    }
                    PeerType::Fetch => {
                        task_set.insert(
                            task_id.clone(),
                            tokio::spawn(fetch_peer::fetch_from_peer(
                                config.clone(),
                                quit.clone(),
                                secp.clone(),
                                client,
                                (task_id.0, task_id.1),
                                db.clone(),
                                task_id.3,
                            )),
                        );
                    }
                }
            }
        }
        INFER_UNIT
    });
    jh
}

mod push_peer;

mod fetch_peer;
