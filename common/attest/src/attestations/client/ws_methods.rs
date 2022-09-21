use super::super::query::Tips;
use super::super::server::protocol;
use super::super::server::protocol::AttestRequest;
use super::AttestationClient;
use super::NotifyOnDrop;
use super::ServiceUrl;
use crate::attestations::server::protocol::LatestTips;
use crate::attestations::server::protocol::Post;
use crate::attestations::server::protocol::SpecificTips;
use crate::control::query::Outcome;
use attest_messages::Envelope;
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::oneshot;
use tokio::sync::Notify;
use tracing::debug;
use tracing::trace;
use tracing::warn;
impl AttestationClient {
    pub async fn get_latest_tips(&self, url: &ServiceUrl) -> Option<Vec<Envelope>> {
        let conn = self.get_conn(url).await;
        let (tx, rx) = oneshot::channel();
        if conn.send_latest_tips((LatestTips {}, tx)).await.is_err() {
            warn!("The channel to enqueue new requests is closed.");
            return None;
        }
        let resp = rx
            .await
            .map_err(|_| {
                warn!("The oneshot::channel to get the reuslt closed without returning a response.")
            })
            .ok()?;
        let tips = resp.0;

        debug!(v=?tips.iter().map(|v|(v.header().height(), v.get_genesis_hash(), v.canonicalized_hash_ref())).collect::<Vec<_>>(), "got tips");
        Some(tips)
    }
    pub async fn get_tips(
        &self,
        mut tips: Tips,
        url: &ServiceUrl,
        use_cache: bool,
    ) -> Option<(Vec<Envelope>, NotifyOnDrop)> {
        trace!("IN get_tips");
        let conn = self.get_conn(url).await;
        if use_cache {
            let mut inflight = self.inflight.lock().await;
            for tip_idx in (0..tips.tips.len()).rev() {
                let h = tips.tips[tip_idx];
                // if we already had this one remove it from the query
                if !inflight.insert(h) {
                    trace!(hash=?h, "skipping entry already in flight");
                    tips.tips.swap_remove(tip_idx);
                } else {
                    trace!(hash=?h, "scheduling fetch of");
                }
            }
        };
        let (tx, rx) = oneshot::channel();
        let send_ok = conn
            .send_specific_tips((SpecificTips { tips: tips.clone() }, tx))
            .await
            .ok();
        let resp_ok = rx.await.ok();

        let cleanup_on_drop = if use_cache {
            let notify = Arc::new(Notify::new());
            spawn({
                let notify = notify.clone();
                let inflight = self.inflight.clone();
                async move {
                    notify.notified().await;
                    let mut inflight = inflight.lock().await;
                    for tip in tips.tips {
                        inflight.remove(&tip);
                    }
                }
            });
            // NotifyOnDrop uses notify_one
            NotifyOnDrop(Some(notify))
        } else {
            NotifyOnDrop(None)
        };
        send_ok?;
        let resp = resp_ok?;
        Some((resp.0, cleanup_on_drop))
    }

    pub async fn post_messages(
        &self,
        envelopes: &Vec<Envelope>,
        url: &ServiceUrl,
    ) -> Option<Vec<Outcome>> {
        let conn = self.get_conn(url).await;
        let (tx, rx) = oneshot::channel();
        conn.send_post((
            Post {
                envelopes: envelopes.clone(),
            },
            tx,
        ))
        .await
        .map_err(|_| {
            warn!("The channel to enqueue new requests is closed.");
        })
        .ok()?;

        let resp = rx
            .await
            .map_err(|_| {
                warn!("The oneshot::channel to get the reuslt closed without returning a response.")
            })
            .ok()?;
        Some(resp.0)
    }
}
