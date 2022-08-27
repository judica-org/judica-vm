use tokio::{
    spawn,
    sync::{Mutex, Notify},
};

use super::*;
pub async fn push_to_peer<C: Verification + 'static>(
    config: Arc<Config>,
    _secp: Arc<Secp256k1<C>>,
    client: AttestationClient,
    service: (String, u16),
    conn: MsgDB,
    shutdown: Arc<AtomicBool>,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let tip_db = Arc::new(Mutex::new(HashMap::new()));
    let new_tips = Arc::new(Notify::new());
    let mut t1 = spawn({
        let service = service.clone();
        let shutdown = shutdown.clone();
        let tip_db = tip_db.clone();
        let client = client.clone();
        let (url, port) = service.clone();
        let new_tips = new_tips.clone();
        async move {
            while !shutdown.load(Ordering::Relaxed) {
                // Get the tips this client claims to have
                let tips = client.get_latest_tips(&url, port).await?;
                debug!(url, port, ?tips, "Fetching saw peer at state");
                {
                    let mut tip_db = tip_db.lock().await;

                    for t in tips {
                        let genesis = t.get_genesis_hash();
                        let previous = tip_db.entry(genesis).or_insert_with(|| {
                            debug!(url, port, ?t, ?genesis, "Inserting Unseen Genesis");
                            t.clone()
                        });
                        // TODO: detect if previous is in our chain?
                        if previous.header.height < t.header.height {
                            debug!(url, port, ?t, ?genesis, "Inserting New Message");
                            *previous = t;
                        } else {
                            debug!(url, port, ?t, ?genesis, "Skipping Old Message");
                        }
                    }
                }
                new_tips.notify_one();
                config
                    .peer_service
                    .timer_override
                    .scan_for_unsent_tips_delay()
                    .await;
            }
            info!(?service, "Shutting Down Push New Envelope Subtask");
            INFER_UNIT
        }
    });
    let mut t2 = spawn({
        let service = service.clone();
        let shutdown = shutdown.clone();
        let tip_db = tip_db.clone();
        let client = client.clone();
        let (url, port) = service.clone();
        let new_tips = new_tips.clone();
        async move {
            while !shutdown.load(Ordering::Relaxed) {
                new_tips.notified().await;
                info!(?service, "Push: TipDB is up to date");
                let to_broadcast = {
                    // get the DB first as it is more contended, and we're OK
                    // waiting on the other thread later
                    let handle = conn.get_handle().await;
                    // if we can't get the lock on tip_db, it means we'll have fresh-er results soon,
                    // so it's fine to wait again.
                    let tip_db = tip_db.try_lock();
                    if let Ok(tip_db) = tip_db {
                        // TODO: Profile if should copy out of tip_db or if hold lock during query.
                        handle.get_connected_messages_newer_than_envelopes(tip_db.values())?
                    } else {
                        continue;
                    }
                };
                if !to_broadcast.is_empty() {
                    info!(?service, n = to_broadcast.len(), "Pushing Messages!");
                    client.post_messages(to_broadcast, &url, port).await?;
                } else {
                    info!(?service, "Erroneous Wakeup");
                }
            }
            info!(?service, "Shutting Down Push Envelope Sending Subtask");
            INFER_UNIT
        }
    });

    let r = tokio::select! {
        a = &mut t1 => {
            info!(?service, error=?a, "New Envelope Detecting Subtask");
            a??
        },
        a = &mut t2 => {
            info!(?service, error=?a, "Envelope Pushing subtask");
            a??
        }
    };
    t1.abort();
    t2.abort();
    Ok(())
}
