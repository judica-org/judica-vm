// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![deny(unused_must_use)]
use super::super::server::protocol;
use super::super::server::tungstenite_client_adaptor;
use super::new_protocol_chan;
use super::AttestationClient;

use super::PeerState;
use super::ProtocolChan;
use super::ProtocolReceiver;
use super::ServiceUrl;
use crate::attestations::client::PENDING_COOKIE;
use crate::attestations::server::protocol::get_my_name;
use crate::globals::Globals;
use reqwest::Client;
use sapio_bitcoin::secp256k1::rand::thread_rng;
use sapio_bitcoin::secp256k1::rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::spawn;
use tokio::task::JoinHandle;
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
            Some(PeerState::Open(p, role)) => {
                if !p.is_closed() {
                    trace!(?svc, ?role, "Client Connection Found to be Open");
                    PeerState::Open(p.clone(), *role)
                } else {
                    trace!(?svc, ?role, "Client Connection Found to be Closed");
                    PeerState::Closed
                }
            }
            Some(PeerState::Pending(c)) => {
                trace!(
                    ?svc,
                    cookie = c,
                    readonly = true,
                    "Client Connection Pending"
                );
                PeerState::Pending(*c)
            }
            None => PeerState::Closed,
        }
    }
    pub async fn set_conn_closed_from_pending(&self, svc: &ServiceUrl, current_cookie: u64) {
        let mut f = self.connections.write().await;
        let x = f
            .get_mut(svc)
            .expect("If we have a cookie the service is present");
        if let PeerState::Pending(u) = x {
            if *u == current_cookie {
                trace!(?svc, "Pending Attempt Failed");
                // rematch for brwchk
                *x = PeerState::Closed;
            }
        }
    }
    pub async fn set_conn_open_prob(
        &self,
        svc: &ServiceUrl,
        prefer_role: Role,
        role: Role,
    ) -> Option<ProtocolReceiver> {
        trace!("CRITICAL TRACE");
        let mut f = self.connections.write().await;
        trace!(?f, "CRITICAL TRACE: ALl the Connection States");
        let mut fresh = false;
        let mut rec = None;
        let ent = f.entry(svc.clone()).or_insert_with(|| {
            trace!(?svc, ?prefer_role, ?role, "Opening a new Conn, first init");
            fresh = true;
            let (a, b) = new_protocol_chan(100);
            rec = Some(b);
            PeerState::Open(a, role)
        });
        trace!(
            ?ent,
            "CRITICAL TRACE:  modified Connection States, should be updated"
        );
        if fresh {
            trace!(?ent, "CRITICAL TRACE:  returning Recv");
            return rec;
        }
        match ent {
            PeerState::Open(_, _r) => {
                trace!(?ent, "CRITICAL TRACE:  returning no Recv NONE");
                return None;
            }
            PeerState::Closed => {
                trace!(
                    ?svc,
                    ?prefer_role,
                    ?role,
                    "Opening a new Conn, previously closed"
                );
                let (a, b) = new_protocol_chan(100);
                *ent = PeerState::Open(a, role);
                trace!(?ent, "CRITICAL TRACE:  returning new Recv");
                return Some(b);
            }
            PeerState::Pending(_) => {
                trace!(
                    ?svc,
                    ?prefer_role,
                    ?role,
                    "Opening a new Conn, previously pending"
                );
                let (a, b) = new_protocol_chan(100);
                *ent = PeerState::Open(a, role);
                trace!(?ent, "CRITICAL TRACE:  returning new Recv");
                return Some(b);
            }
        }
        rec
    }
    pub async fn set_conn_pending(&self, svc: &ServiceUrl, force: bool) -> Option<u64> {
        let d = { Duration::from_millis(thread_rng().gen_range(0, 1000)) };
        tokio::time::sleep(d).await;
        let mut ret_cookie = None;
        let mut f = self.connections.write().await;
        let r = f.entry(svc.clone()).or_insert_with(|| {
            let cookie = PENDING_COOKIE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            trace!(?svc, cookie, "New Client Connection, Pending");
            ret_cookie = Some(cookie);
            PeerState::Pending(cookie)
        });
        if ret_cookie.is_some() {
            return ret_cookie;
        }
        match r {
            PeerState::Closed => {
                trace!(?svc, "Client Connection Closed");
                let cookie = PENDING_COOKIE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                *r = PeerState::Pending(cookie);
                Some(cookie)
            }
            PeerState::Open(ref mut p, existing_role) => {
                if !p.is_closed() {
                    trace!(?svc, role=?existing_role, "Client Connection Found to be Open");
                    None
                } else {
                    trace!(?svc, "Client Connection Found to be Closed");
                    let cookie = PENDING_COOKIE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    *r = PeerState::Pending(cookie);
                    Some(cookie)
                }
            }
            PeerState::Pending(c) => {
                if force {
                    trace!(
                        ?svc,
                        cookie = c,
                        "Client Connection Found to be Pending, but force used to overwrite"
                    );
                    let cookie = PENDING_COOKIE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    *r = PeerState::Pending(cookie);
                    Some(cookie)
                } else {
                    trace!(
                        ?svc,
                        cookie = c,
                        "Client Connection Pending, force not enabled"
                    );
                    None
                }
            }
        }
    }

    pub async fn get_conn(&self, svc: &ServiceUrl) -> ProtocolChan {
        trace!(?svc, "Requesting Connection");
        let mut cookie = None;
        let mut ojh: Option<JoinHandle<()>> = None;
        loop {
            let finished_or_not_owner = if let Some(ref jh) = ojh {
                if jh.is_finished() {
                    ojh = None;
                    true
                } else {
                    false
                }
            } else {
                true
            };
            let s = self.conn_already_exists(svc).await;

            let waiting_in = get_my_name(&self.g).await.unwrap();
            trace!(?waiting_in, peer_state = ?s, ?svc, "Current Peer State");
            match s {
                PeerState::Open(s, _r) => return s,

                PeerState::Pending(current_cookie) => {
                    // check if finished and owner
                    if finished_or_not_owner && Some(current_cookie) == cookie {
                        trace!(?svc, "Terminating Attempt Request");
                        self.set_conn_closed_from_pending(svc, current_cookie).await;
                    } else {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
                PeerState::Closed => {
                    trace!(?svc, "Requesting Cookie for Pending Initialization");
                    cookie = self.set_conn_pending(svc, true).await;
                    if cookie.is_none() {
                        trace!(?svc, "No Cookie Gotten");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    };
                    // Otherwise, we are supposed to set up a connection...
                    let svc_url = svc.to_string();
                    trace!(svc_url, "Must Create a New P2P Channel");
                    let g = self.g.clone();
                    let gss = self.gss.clone();
                    let db = self.db.clone();
                    let svc = svc.clone();
                    ojh = Some(spawn(async move {
                        let socket = loop {
                            if let Ok(socket) =
                                tungstenite_client_adaptor::ClientWebSocket::connect(
                                    &g,
                                    svc_url.clone(),
                                )
                                .await
                            {
                                tracing::info!(
                                    ?svc_url,
                                    role = ?Role::Client,
                                    "Socket Opened To"
                                );
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
                        trace!(?res, role=?Role::Client,"websocket terminated");
                    }));
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
