use super::super::server::protocol;
use super::super::server::tungstenite_client_adaptor;
use super::new_protocol_chan;
use super::AttestationClient;
use super::OpenState;
use super::ProtocolChan;
use super::ServiceUrl;
use crate::globals::Globals;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::spawn;

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
    pub async fn conn_already_exists_or_create(&self, svc: &ServiceUrl) -> OpenState {
        if let Some(ch) = self.conn_already_exists(svc).await {
            return OpenState::Already(ch);
        }

        {
            let mut f = self.connections.write().await;
            let e = f.entry(svc.clone());
            let mut open_state = OpenState::Unknown;
            e.and_modify(|prior_tx| {
                if prior_tx.is_closed() {
                    trace!(?svc, "Removing Closed Connection");
                    let (a, b) = new_protocol_chan(100);
                    *prior_tx = a.clone();
                    open_state = OpenState::Newly(a, b);
                } else {
                    trace!(
                        ?svc,
                        "Client Connection Found to be Opened by some other Thread"
                    );
                    open_state = OpenState::Already(prior_tx.clone());
                }
            })
            .or_insert_with(|| {
                let (a, b) = new_protocol_chan(100);
                open_state = OpenState::Newly(a.clone(), b);
                a
            });
            if let OpenState::Unknown = open_state {
                unreachable!("Must have Been Set");
            }
            open_state
        }
    }
    pub async fn get_conn(&self, svc: &ServiceUrl) -> ProtocolChan {
        let s = self.conn_already_exists_or_create(svc).await;
        let svc_url = svc.to_string();
        match s {
            OpenState::Newly(tx, rx) => {
                let g = self.g.clone();
                let gss = self.gss.clone();
                let db = self.db.clone();
                spawn(async move {
                    let socket = loop {
                        if let Ok(socket) =
                            tungstenite_client_adaptor::ClientWebSocket::connect(&g, svc_url.clone()).await
                        {
                            break socket;
                        }
                        tokio::time::sleep(Duration::from_secs(1));
                    };
                    let res =
                        protocol::run_protocol(g, socket, gss, db, Role::Client, Some(rx)).await;
                    trace!(?res, role=?Role::Server,"socket quit");
                });
                tx
            }
            OpenState::Already(tx) => tx,
            OpenState::Unknown => unreachable!("Must have been set"),
        }
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
