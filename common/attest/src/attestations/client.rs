use super::server::{
    protocol::{self, GlobalSocketState},
    tungstenite_client_adaptor::{self},
};
use crate::globals::Globals;
use attest_database::connection::MsgDB;
use attest_messages::CanonicalEnvelopeHash;
use reqwest::Client;
use std::{
    collections::{BTreeSet, HashMap},
    fmt::Display,
    sync::Arc,
};
use tokio::sync::{mpsc::Sender, oneshot, Mutex, Notify, RwLock};
type ProtocolChan = Sender<(
    protocol::AttestRequest,
    oneshot::Sender<protocol::AttestResponse>,
)>;

mod connection_creation;
// Methods
mod http_methods;
mod ws_methods;

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
pub enum OpenState {
    Already,
    Newly,
    Closed,
}
