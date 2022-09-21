use super::super::query::Tips;
use super::super::server::protocol;
use super::super::server::protocol::AttestRequest;
use super::AttestationClient;
use super::NotifyOnDrop;
use super::ServiceUrl;
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
    pub async fn get_latest_tips(&self, url: &String, port: u16) -> Option<Vec<Envelope>> {
        let conn = self.get_conn(ServiceUrl(url.clone(), port)).await;
        let (tx, rx) = oneshot::channel();
        if conn.send((AttestRequest::LatestTips, tx)).await.is_err() {
            warn!("The channel to enqueue new requests is closed.");
            return None;
        }
        let resp = rx.await.ok();

        match resp {
            Some(protocol::AttestResponse::LatestTips(tips)) => {
                debug!(v=?tips.iter().map(|v|(v.header().height(), v.get_genesis_hash(), v.canonicalized_hash_ref())).collect::<Vec<_>>(), "got tips");
                Some(tips)
            }
            Some(
                protocol::AttestResponse::PostResult(_) | protocol::AttestResponse::SpecificTips(_),
            ) => {
                warn!(?resp, "Invalid Response Type");
                None
            }
            None => {
                warn!(
                    "The oneshot::channel to get the reuslt closed without returning a response."
                );
                None
            }
        }
    }
    pub async fn get_tips(
        &self,
        mut tips: Tips,
        url: &String,
        port: u16,
        use_cache: bool,
    ) -> Option<(Vec<Envelope>, NotifyOnDrop)> {
        trace!("IN get_tips");
        let conn = self.get_conn(ServiceUrl(url.clone(), port)).await;
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
            .send((AttestRequest::SpecificTips(tips.clone()), tx))
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
        match resp_ok? {
            protocol::AttestResponse::SpecificTips(t) => Some((t, cleanup_on_drop)),
            resp @ protocol::AttestResponse::LatestTips(_)
            | resp @ protocol::AttestResponse::PostResult(_) => {
                warn!(?resp, "Invalid Response Type");
                None
            }
        }
    }

    pub async fn post_messages(
        &self,
        envelopes: &Vec<Envelope>,
        url: &String,
        port: u16,
    ) -> Option<Vec<Outcome>> {
        let conn = self.get_conn(ServiceUrl(url.clone(), port)).await;
        let (tx, rx) = oneshot::channel();
        if conn
            .send((AttestRequest::Post(envelopes.clone()), tx))
            .await
            .is_err()
        {
            warn!("The channel to enqueue new requests is closed.");
            return None;
        }
        let resp = rx.await.ok();
        match resp {
            None => {
                warn!(
                    "The oneshot::channel to get the reuslt closed without returning a response."
                );
                None
            }
            Some(
                protocol::AttestResponse::LatestTips(_) | protocol::AttestResponse::SpecificTips(_),
            ) => {
                warn!(?resp, "Invalid Response Type");
                None
            }
            Some(protocol::AttestResponse::PostResult(o)) => {
                debug!("successfully posted messages");
                Some(o)
            }
        }
    }
}
