use std::collections::BTreeSet;

use attest_messages::{CanonicalEnvelopeHash, Envelope};
use tokio::{
    spawn,
    sync::{Mutex, Notify},
};
use tracing::{trace, warn};

use super::*;
pub async fn push_to_peer<C: Verification + 'static>(
    g: Arc<Globals>,
    _secp: Arc<Secp256k1<C>>,
    client: AttestationClient,
    service: (String, u16),
    conn: MsgDB,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    #[derive(Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Copy, Clone)]
    struct GenesisHash(CanonicalEnvelopeHash);
    impl From<&Envelope> for GenesisHash {
        fn from(e: &Envelope) -> Self {
            GenesisHash(e.get_genesis_hash())
        }
    }
    let tip_tracker: Arc<Mutex<HashMap<GenesisHash, _>>> = Arc::new(Mutex::new(HashMap::new()));
    let new_tips = Arc::new(Notify::new());
    let mut t1 = spawn({
        let service = service.clone();
        let g = g.clone();
        let tip_tracker = tip_tracker.clone();
        let client = client.clone();
        let (url, port) = service.clone();
        let new_tips = new_tips.clone();
        async move {
            while !g.shutdown.should_quit() {
                // Get the tips this client claims to have
                let tips = client.get_latest_tips(&url, port).await?;
                trace!(
                    url,
                    port,
                    task = "PUSH::tip_tracker",
                    ?tips,
                    "Fetching saw peer at state"
                );
                debug!(url, port,
                        task = "PUSH::tip_tracker",
                    tips=?tips.iter().map(|t| (t.get_genesis_hash(), t.canonicalized_hash_ref())).collect::<Vec<_>>(), "Fetching saw peer at state");
                {
                    let mut tip_tracker = tip_tracker.lock().await;

                    for t in tips {
                        let genesis = GenesisHash::from(&t);
                        let previous = tip_tracker.entry(genesis).or_insert_with(|| {
                            debug!(url, port, ?t, ?genesis, "Inserting Unseen Genesis");
                            t.clone()
                        });
                        // TODO: detect if previous is in our chain?
                        match t.header().height().cmp(&previous.header().height()) {
                            std::cmp::Ordering::Less => {
                                warn!(
                                    url,
                                    port,
                                    ?t,
                                    ?genesis,
                                    task = "PUSH::tip_tracker",
                                    "Got Old Message"
                                );
                            }
                            std::cmp::Ordering::Equal => {
                                if t != *previous {
                                    warn!(url, port, ?t, ?previous, ?genesis, "Conflict Seen");
                                } else {
                                    info!(url, port, hash=?t.canonicalized_hash_ref(), ?genesis, task="PUSH::tip_tracker", "Nothing New");
                                    trace!(url, port, ?t, task = "PUSH::tip_tracker", "No Updates");
                                }
                            }
                            std::cmp::Ordering::Greater => {
                                trace!(
                                    url,
                                    port,
                                    ?t,
                                    ?genesis,
                                    task = "PUSH::tip_tracker",
                                    "Advancing Tip"
                                );
                                info!(
                                    url,
                                    port,
                                    height = t.header().height(),
                                    old_height = previous.header().height(),
                                    task = "PUSH::tip_tracker",
                                    ?genesis,
                                    "Advancing Tip"
                                );
                                *previous = t;
                            }
                        }
                    }
                }
                new_tips.notify_one();
                g.config
                    .peer_service
                    .timer_override
                    .scan_for_unsent_tips_delay()
                    .await;
            }
            INFER_UNIT.map(|_| format!("Shutdown Graceful: {}", g.shutdown.should_quit()))
        }
    });
    let mut t2 = spawn({
        let service = service.clone();
        let g = g.clone();
        let tip_tracker = tip_tracker.clone();
        let client = client.clone();
        let (url, port) = service.clone();
        let new_tips = new_tips.clone();
        async move {
            while !g.shutdown.should_quit() {
                new_tips.notified().await;
                let to_broadcast = {
                    // get the DB first as it is more contended, and we're OK
                    // waiting on the other thread later
                    let handle = conn.get_handle().await;
                    // if we can't get the lock on tip_tracker, it means we'll have fresh-er results soon,
                    // so it's fine to wait again.
                    let tip_tracker = tip_tracker.try_lock();
                    if let Ok(tip_tracker) = tip_tracker {
                        // TODO: Profile if should copy out of tip_tracker or if hold lock during query.
                        let mut msgs = handle
                            .get_connected_messages_newer_than_envelopes(tip_tracker.values())?;
                        let gen = handle.get_all_genesis()?;
                        drop(handle);

                        let following_chains: BTreeSet<_> =
                            msgs.iter().map(|e| e.get_genesis_hash()).collect();

                        let mut genesis: Vec<_> = gen
                            .into_iter()
                            .map(|e| Some((e.get_genesis_hash(), e)))
                            .collect();
                        for o in genesis.iter_mut() {
                            match o {
                                Some((h, _e)) => {
                                    if following_chains.contains(&h) {
                                        *o = None;
                                    }
                                }
                                None => {}
                            }
                        }
                        debug!(unknown_chains = ?genesis);
                        msgs.extend(genesis.into_iter().flatten().map(|(_a, b)| b));
                        msgs
                    } else {
                        continue;
                    }
                };
                if !to_broadcast.is_empty() {
                    info!(?service, task = "PUSH::broadcast", n = to_broadcast.len());
                    trace!(?service, task="PUSH::broadcast", msgs = ?to_broadcast);

                    let res = client.post_messages(&to_broadcast, &url, port).await?;

                    info!(
                        accepted = res.iter().filter(|s| s.success).count(),
                        out_of = to_broadcast.len(),
                        task = "PUSH",
                        ?service
                    );
                } else {
                    info!(?service, task = "PUSH", "No Work to Do");
                }
            }
            INFER_UNIT.map(|_| format!("Shutdown Graceful: {}", g.shutdown.should_quit()))
        }
    });

    let _r = tokio::select! {
        a = &mut t1 => {
            t2.abort();
            match &a {
                Ok(r) => match r {
                    Ok(msg) => {
                        info!(?service, task = "PUSH::tip_tracker", event = "SHUTDOWN", msg);
                    },
                    Err(e) => {
                        warn!(?service, task="PUSH::tip_tracker", event="SHUTDOWN", err=?e, "Fail");
                    },
                },
                Err(e) => {
                    warn!(?service, task="PUSH::tip_tracker", event="SHUTDOWN", err=?e, "Fail");
                },
            };
            a??;
        },
        a = &mut t2 => {
            t1.abort();
            match &a {
                Ok(r) => match r {
                    Ok(msg) => {
                        info!(?service, task = "PUSH::broadcast", event = "SHUTDOWN", msg);
                    },
                    Err(e) => {
                        warn!(?service, task="PUSH::broadcast", event="SHUTDOWN", err=?e, "Fail");
                    },
                },
                Err(e) => {
                    warn!(?service, task="PUSH::broadcast", event="SHUTDOWN", err=?e, "Fail");
                },
            }
            a??;
        }
    };
    Ok(())
}
