use tokio::{
    spawn,
    sync::{Mutex, Notify},
};

use super::*;
pub async fn push_to_peer<C: Verification + 'static>(
    _secp: Arc<Secp256k1<C>>,
    client: AttestationClient,
    service: (String, u16),
    conn: MsgDB,
    shutdown: Arc<AtomicBool>,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let tip_db = Arc::new(Mutex::new(HashMap::new()));
    let new_tips = Arc::new(Notify::new());
    let mut t1 = spawn({
        let shutdown = shutdown.clone();
        let tip_db = tip_db.clone();
        let client = client.clone();
        let (url, port) = service.clone();
        let new_tips = new_tips.clone();
        async move {
            while shutdown.load(Ordering::Relaxed) {
                // Get the tips this client claims to have
                let tips = client.get_latest_tips(&url, port).await?;
                let mut any_new = false;
                {
                    let mut tip_db = tip_db.lock().await;
                    for t in tips {
                        let previous = tip_db.entry(t.header.genesis).or_insert_with(|| {
                            any_new = true;
                            t.clone()
                        });
                        // TODO: detect if previous is in our chain?
                        if previous.header.height < t.header.height {
                            *previous = t;
                            any_new = true;
                        }
                    }
                }
                if any_new {
                    new_tips.notify_one();
                }
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            INFER_UNIT
        }
    });
    let mut t2 = spawn({
        let shutdown = shutdown.clone();
        let tip_db = tip_db.clone();
        let client = client.clone();
        let (url, port) = service.clone();
        let new_tips = new_tips.clone();
        async move {
            while shutdown.load(Ordering::Relaxed) {
                new_tips.notified().await;
                let to_broadcast = {
                    let handle = conn.get_handle().await;
                    let tip_db = tip_db.lock().await;
                    // TODO: Profile if should copy out of tip_db or if hold lock during query.
                    handle.get_connected_messages_newer_than_envelopes(tip_db.values())?
                };
                client.post_messages(to_broadcast, &url, port).await?;
            }
            INFER_UNIT
        }
    });

    let _ = tokio::select! {
        a = &mut t1 => {
            a??
        },
        a = &mut t2 => {
            a??
        }
    };
    Ok(())
}
