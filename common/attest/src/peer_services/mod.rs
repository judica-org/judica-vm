use tokio::{
    sync::{mpsc::Receiver, oneshot::Sender},
    time::MissedTickBehavior,
};

use attest_util::INFER_UNIT;

use crate::attestations::client::AttestationClient;

use super::*;

#[derive(Hash, Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Serialize, Deserialize)]
pub enum PeerType {
    Push,
    Fetch,
}

pub enum PeerQuery {
    RunningTasks(Sender<Vec<(String, u16, PeerType)>>),
}
pub fn startup(
    config: Arc<Config>,
    db: MsgDB,
    quit: Arc<AtomicBool>,
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
        let client = AttestationClient(inner_client);
        let secp = Arc::new(Secp256k1::new());
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut task_set: HashMap<(String, u16, PeerType), JoinHandle<Result<(), _>>> =
            HashMap::new();
        'outer: loop {
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
                        v[0] = Some((p.service_url.clone(), p.port, PeerType::Fetch))
                    }
                    if p.push_to {
                        v[1] = Some((p.service_url, p.port, PeerType::Push))
                    }
                    v
                })
                .flatten()
                .collect();
            // Drop anything that is finished, we will re-add it later if still in create_services
            task_set.retain(|_k, v| !v.is_finished());
            // If it is no longer in our services DB, drop it / disconnect, and also
            // remove it from our to create set (only if it is in the to retain set, mind you)
            task_set.retain(
                |service_id, service| match create_services.take(service_id) {
                    Some(_) => true,
                    None => {
                        service.abort();
                        false
                    }
                },
            );
            // Open connections to all services on the list and put into our task set.
            for url in create_services.into_iter() {
                let client = client.clone();
                match url.2 {
                    PeerType::Push => {
                        task_set.insert(
                            url.clone(),
                            tokio::spawn(push_peer::push_to_peer(
                                secp.clone(),
                                client,
                                (url.0, url.1),
                                db.clone(),
                                quit.clone(),
                            )),
                        );
                    }
                    PeerType::Fetch => {
                        task_set.insert(
                            url.clone(),
                            tokio::spawn(fetch_peer::fetch_from_peer(
                                secp.clone(),
                                client,
                                (url.0, url.1),
                                db.clone(),
                            )),
                        );
                    }
                }
            }
        }
        // INFER_UNIT
    });
    jh
}

mod push_peer;

mod fetch_peer;
