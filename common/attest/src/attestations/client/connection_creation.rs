use super::super::server::protocol;
use super::super::server::tungstenite_client_adaptor;
use super::new_protocol_chan;
use super::AttestationClient;
use super::OpenState;
use super::PeerState;
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
    pub async fn conn_already_exists(&self, svc: &ServiceUrl) -> PeerState {
        let f = self.connections.read().await;
        match f.get(svc) {
            Some(PeerState::Closed) => {
                trace!(?svc, "Client Connection Closed");
                PeerState::Closed
            }
            Some(PeerState::Open(p)) => {
                if !p.is_closed() {
                    trace!(?svc, "Client Connection Found to be Open");
                    PeerState::Open(p.clone())
                } else {
                    trace!(?svc, "Client Connection Found to be Closed");
                    PeerState::Closed
                }
            }
            Some(PeerState::Pending) => {
                trace!(?svc, "Client Connection Pending");
                PeerState::Pending
            }
            None => PeerState::Closed,
        }
    }
    pub async fn set_conn_pending(&self, svc: &ServiceUrl) -> bool {
        let mut f = self.connections.write().await;
        let x = f.get_mut(svc);
        match x {
            Some(v @ PeerState::Closed) => {
                trace!(?svc, "Client Connection Closed");
                *v = PeerState::Pending;
                true
            }
            Some(PeerState::Open(ref mut p)) => {
                if !p.is_closed() {
                    trace!(?svc, "Client Connection Found to be Open");
                    false
                } else {
                    trace!(?svc, "Client Connection Found to be Closed");
                    x.map(|r| *r = PeerState::Pending);
                    true
                }
            }
            Some(PeerState::Pending) => {
                trace!(?svc, "Client Connection Pending");
                false
            }
            None => {
                f.insert(svc.clone(), PeerState::Pending);
                true
            }
        }
    }

    pub async fn conn_already_exists_or_create(&self, svc: &ServiceUrl) -> OpenState {
        if let PeerState::Open(ch) = self.conn_already_exists(svc).await {
            return OpenState::Already(ch);
        }

        {
            let mut f = self.connections.write().await;
            let e = f.entry(svc.clone());
            let mut open_state = OpenState::Unknown;
            e.and_modify(|prior_tx| match prior_tx {
                PeerState::Open(a) => {
                    if a.is_closed() {
                        trace!(?svc, "Removing + Reopening Dirty Closed Connection");
                        let (new_a, b) = new_protocol_chan(100);
                        *prior_tx = PeerState::Open(new_a.clone());
                        open_state = OpenState::Newly(new_a, b);
                    } else {
                        trace!(
                            ?svc,
                            "Client Connection Found to be Opened by some other Thread"
                        );
                        open_state = OpenState::Already(a.clone());
                    }
                }
                PeerState::Closed => {
                    trace!(?svc, "Removing + Reopening Clean Closed Connection");
                    let (a, b) = new_protocol_chan(100);
                    *prior_tx = PeerState::Open(a.clone());
                    open_state = OpenState::Newly(a, b);
                }
                PeerState::Pending => {
                    trace!(?svc, "Finishing Pending Open Connection");
                    let (a, b) = new_protocol_chan(100);
                    *prior_tx = PeerState::Open(a.clone());
                    open_state = OpenState::Newly(a, b);
                }
            })
            .or_insert_with(|| {
                let (a, b) = new_protocol_chan(100);
                open_state = OpenState::Newly(a.clone(), b);
                PeerState::Open(a)
            });
            if let OpenState::Unknown = open_state {
                unreachable!("Must have Been Set");
            }
            open_state
        }
    }
    pub async fn get_conn(&self, svc: &ServiceUrl) -> ProtocolChan {
        loop {
            let s = self.conn_already_exists(svc).await;
            match s {
                PeerState::Open(s) => return s,
                PeerState::Pending => tokio::time::sleep(Duration::from_secs(1)).await,
                PeerState::Closed => {
                    if !self.set_conn_pending(svc).await {
                        continue;
                    }
                    // Otherwise, we are supposed to set up a connection...
                    let svc_url = svc.to_string();
                    trace!(svc_url, "Must Create a New P2P Channel");
                    let g = self.g.clone();
                    let gss = self.gss.clone();
                    let db = self.db.clone();
                    let svc = svc.clone();
                    spawn(async move {
                        let socket = loop {
                            if let Ok(socket) =
                                tungstenite_client_adaptor::ClientWebSocket::connect(
                                    &g,
                                    svc_url.clone(),
                                )
                                .await
                            {
                                break socket;
                            }
                            tracing::debug!(
                                ?svc_url,
                                role = ?Role::Client,
                                "Retrying Opening Socket To"
                            );
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        };
                        let res =
                            protocol::run_protocol(g, socket, gss, db, Role::Client, Some(svc))
                                .await;
                        trace!(?res, role=?Role::Client,"socket quit");
                    });
                }
            }
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
