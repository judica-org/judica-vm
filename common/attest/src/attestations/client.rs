use super::query::Tips;
use crate::control::query::Outcome;
use attest_messages::{CanonicalEnvelopeHash, Envelope};
use reqwest::Client;
use std::{collections::BTreeSet, sync::Arc};
use tokio::{
    spawn,
    sync::{Mutex, Notify},
};
use tracing::{debug, trace};

#[derive(Clone)]
pub struct AttestationClient {
    client: Client,
    inflight: Arc<Mutex<BTreeSet<CanonicalEnvelopeHash>>>,
}

#[derive(Debug)]
pub struct NotifyOnDrop(Option<Arc<Notify>>);
impl NotifyOnDrop {
    pub fn empty() -> Self {
        Self(None)
    }
}
impl Drop for NotifyOnDrop {
    fn drop(&mut self) {
        self.0.as_ref().map(|n| n.notify_one());
    }
}
impl AttestationClient {
    pub fn new(client: Client) -> Self {
        AttestationClient {
            client,
            inflight: Default::default(),
        }
    }
    pub async fn get_latest_tips(
        &self,
        url: &String,
        port: u16,
    ) -> Result<Vec<Envelope>, reqwest::Error> {
        let resp: Vec<Envelope> = self
            .client
            .get(format!("http://{}:{}/newest_tips", url, port))
            .send()
            .await?
            .json()
            .await?;
        debug!(v=?resp.iter().map(|v|(v.header.height, v.get_genesis_hash(), v.canonicalized_hash_ref().unwrap())).collect::<Vec<_>>());
        Ok(resp)
    }
    pub async fn get_tips(
        &self,
        mut tips: Tips,
        url: &String,
        port: u16,
        use_cache: bool,
    ) -> Result<(Vec<Envelope>, NotifyOnDrop), reqwest::Error> {
        trace!("IN get_tips");
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
        }
        let req = self
            .client
            .get(format!("http://{}:{}/tips", url, port))
            .json(&tips);
        let notify = Arc::new(Notify::new());
        let notified = notify.notified();
        if use_cache {
            spawn({
                let notify = notify.clone();
                let inflight = self.inflight.clone();
                async move {
                    let waiter = notify.notified();
                    notify.notify_one();
                    waiter.await;
                    let mut inflight = inflight.lock().await;
                    for tip in tips.tips {
                        inflight.remove(&tip);
                    }
                }
            });
        }
        notified.await;
        let notify_on = NotifyOnDrop(Some(notify));
        match req.send().await {
            Ok(s) => match s.json().await {
                Ok(j) => Ok((j, notify_on)),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        }
    }

    pub async fn post_messages(
        &self,
        envelopes: &Vec<Envelope>,
        url: &String,
        port: u16,
    ) -> Result<Vec<Outcome>, reqwest::Error> {
        let resp = self
            .client
            .post(format!("http://{}:{}/msg", url, port))
            .json(envelopes)
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }
}
