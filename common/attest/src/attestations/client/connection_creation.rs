use super::super::server::protocol;
use super::super::server::tungstenite_client_adaptor;
use super::AttestationClient;
use super::OpenState;
use super::ProtocolChan;
use super::ServiceUrl;
use crate::globals::Globals;
use reqwest::Client;
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::mpsc::channel;
use tokio_tungstenite::tungstenite::protocol::Role;
use tracing::trace;

impl AttestationClient {
    pub async fn conn_already_exists(&self, svc: &ServiceUrl) -> Option<ProtocolChan> {
        let f = self.connections.read().await;
        if let Some(s) = f.get(svc) {
            if !s.is_closed() {
                trace!(?svc, "Client Connection Found to be Open");
                return Some(s.clone());
            } else {
                trace!(?svc, "Client Connection Found to be Closed");
            }
        } else {
            trace!(?svc, "Client Connection Doesn't Exist");
        }
        None
    }
    pub async fn conn_already_exists_or_create(
        &self,
        svc: &ServiceUrl,
        tx: ProtocolChan,
    ) -> (OpenState, ProtocolChan) {
        if let Some(ch) = self.conn_already_exists(svc).await {
            return (OpenState::Already, ch);
        }

        {
            let mut f = self.connections.write().await;
            let e = f.entry(svc.clone());
            let mut ret = (OpenState::Newly, tx.clone());
            e.and_modify(|prior_tx| {
                if tx.is_closed() {
                    trace!(?svc, "Removing Closed Connection");
                    *prior_tx = tx.clone();
                } else {
                    trace!(
                        ?svc,
                        "Client Connection Found to be Opened by some other Thread"
                    );
                    ret = (OpenState::Already, prior_tx.clone());
                }
            })
            .or_insert_with(|| tx.clone());
            ret
        }
    }
    pub async fn get_conn(&self, svc: ServiceUrl) -> ProtocolChan {
        let (tx, rx) = channel(100);
        let (s, tx) = self.conn_already_exists_or_create(&svc, tx).await;
        let svc_url = svc.to_string();
        if let OpenState::Newly = s {
            let g = self.g.clone();
            let gss = self.gss.clone();
            let db = self.db.clone();
            spawn(async move {
                let socket = tungstenite_client_adaptor::ClientWebSocket::connect(svc_url).await;
                let res = protocol::run_protocol(g, socket, gss, db, Role::Client, Some(rx)).await;
                trace!(?res, role=?Role::Server,"socket quit");
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
}
