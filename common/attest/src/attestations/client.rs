use super::{
    query::Tips,
    server::{
        protocol::{self, AttestRequest, GlobalSocketState},
        tungstenite_client_adaptor::{self},
    },
};
use crate::{control::query::Outcome, globals::Globals};
use attest_database::connection::MsgDB;
use attest_messages::{CanonicalEnvelopeHash, Envelope};
use reqwest::Client;
use std::{
    collections::{BTreeSet, HashMap},
    fmt::Display,
    sync::Arc,
};
use tokio::{
    spawn,
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        oneshot, Mutex, Notify, RwLock,
    },
};
use tokio_tungstenite::tungstenite::protocol::Role;
use tracing::{debug, trace};
type ProtocolChan = UnboundedSender<(
    protocol::AttestRequest,
    oneshot::Sender<protocol::AttestResponse>,
)>;
use tracing::warn;

#[derive(Clone)]
pub struct AttestationClient {
    client: Client,
    inflight: Arc<Mutex<BTreeSet<CanonicalEnvelopeHash>>>,
    connections: Arc<RwLock<HashMap<ServiceUrl, ProtocolChan>>>,
    g: Arc<Globals>,
    db: MsgDB,
    gss: GlobalSocketState,
}

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct ServiceUrl(pub String, pub u16);
impl Display for ServiceUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ws://{}:{}/socket", self.0, self.1)
    }
}
impl std::fmt::Debug for ServiceUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ServiceUrl")
            .field(&self.to_string())
            .finish()
    }
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
        if let Some(n) = self.0.as_ref() {
            n.notify_one()
        }
    }
}
impl AttestationClient {
    pub async fn conn_already_exists_or_create(&self, svc: &ServiceUrl, tx: ProtocolChan) -> bool {
        {
            let f = self.connections.read().await;
            if let Some(s) = f.get(svc) {
                if !s.is_closed() {
                    trace!(?svc, "Client Connection Found to be Open");
                    return true;
                } else {
                    trace!(?svc, "Client Connection Found to be Closed");
                }
            } else {
                trace!(?svc, "Client Connection Doesn't Exist");
            }
        }

        {
            let mut f = self.connections.write().await;
            let e = f.entry(svc.clone());
            let mut ret = false;
            e.and_modify(|tx| {
                if tx.is_closed() {
                    trace!(?svc, "Removing Closed Connection");
                    *tx = tx.clone();
                } else {
                    trace!(
                        ?svc,
                        "Client Connection Found to be Opened by some other Thread"
                    );
                    ret = true;
                }
            })
            .or_insert_with(|| tx.clone());
            ret
        }
    }
    pub async fn get_conn(&self, svc: ServiceUrl) -> ProtocolChan {
        {
            let f = self.connections.read().await;
            if let Some(s) = f.get(&svc) {
                if !s.is_closed() {
                    trace!(?svc, "Client Connection Found to be Open");
                    return s.clone();
                } else {
                    trace!(?svc, "Client Connection Found to be Closed");
                }
            } else {
                trace!(?svc, "Client Connection Doesn't Exist");
            }
        }
        let mut f = self.connections.write().await;
        let e = f.entry(svc.clone());
        match e {
            std::collections::hash_map::Entry::Occupied(s) if !s.get().is_closed() => {
                trace!(
                    ?svc,
                    "Client Connection Found to be Opened by some other Thread"
                );
                return s.get().clone();
            }
            std::collections::hash_map::Entry::Occupied(s) => {
                trace!(?svc, "Removing Closed Connection");
                s.remove();
            }
            _ => {}
        }

        let (tx, rx) = unbounded_channel();
        let svc_url = svc.to_string();
        f.entry(svc).or_insert(tx.clone());
        drop(f);
        {
            let g = self.g.clone();
            let gss = self.gss.clone();
            let db = self.db.clone();
            spawn(async move {
                let socket = tungstenite_client_adaptor::ClientWebSocket::connect(svc_url).await;
                protocol::run_protocol(g, socket, gss, db, Role::Client, Some(rx)).await
            });
        }

        tx
    }
    pub fn new(client: Client, g: Arc<Globals>) -> Self {
        AttestationClient {
            client,
            inflight: Default::default(),
            connections: Default::default(),
            db: g.msg_db.clone(),
            gss: g.socket_state.clone(),
            g,
        }
    }
    pub async fn get_latest_tips(&self, url: &String, port: u16) -> Option<Vec<Envelope>> {
        let conn = self.get_conn(ServiceUrl(url.clone(), port)).await;
        let (tx, rx) = oneshot::channel();
        conn.send((AttestRequest::LatestTips, tx)).ok()?;
        let resp = rx.await.ok()?;
        match resp {
            protocol::AttestResponse::LatestTips(tips) => {
                debug!(v=?tips.iter().map(|v|(v.header().height(), v.get_genesis_hash(), v.canonicalized_hash_ref())).collect::<Vec<_>>());
                Some(tips)
            }
            protocol::AttestResponse::PostResult(_) | protocol::AttestResponse::SpecificTips(_) => {
                warn!(?resp, "Invalid Response Type");
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
        conn.send((AttestRequest::Post(envelopes.clone()), tx))
            .ok()?;
        let resp = rx.await.ok()?;
        match resp {
            protocol::AttestResponse::LatestTips(_) | protocol::AttestResponse::SpecificTips(_) => {
                warn!(?resp, "Invalid Response Type");
                None
            }
            protocol::AttestResponse::PostResult(o) => Some(o),
        }
    }

    pub async fn authenticate(
        &self,
        secret: &[u8; 32],
        url: &String,
        port: u16,
    ) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("http://{}:{}/authenticate", url, port))
            .json(secret)
            .send()
            .await?
            .json()
            .await
    }
}
