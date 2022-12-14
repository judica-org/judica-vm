// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::server::protocol::{self, GlobalSocketState};
use crate::globals::Globals;
use attest_database::connection::MsgDB;
use attest_messages::CanonicalEnvelopeHash;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    fmt::Display,
    sync::{atomic::AtomicU64, Arc},
};
use tokio::sync::{
    mpsc::{error::SendError, unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot, Mutex, Notify, RwLock,
};
use tokio_tungstenite::tungstenite::protocol::Role;
use tracing::warn;
type LatestTipsT = (
    protocol::LatestTips,
    oneshot::Sender<protocol::LatestTipsResponse>,
);
type SpecificTipsT = (
    protocol::SpecificTips,
    oneshot::Sender<protocol::SpecificTipsResponse>,
);

pub enum AnySender {
    LatestTips(oneshot::Sender<protocol::LatestTipsResponse>),
    Post(oneshot::Sender<protocol::PostResponse>),
    SpecificTips(oneshot::Sender<protocol::SpecificTipsResponse>),
}
impl From<oneshot::Sender<protocol::SpecificTipsResponse>> for AnySender {
    fn from(c: oneshot::Sender<protocol::SpecificTipsResponse>) -> Self {
        AnySender::SpecificTips(c)
    }
}
impl From<oneshot::Sender<protocol::PostResponse>> for AnySender {
    fn from(c: oneshot::Sender<protocol::PostResponse>) -> Self {
        AnySender::Post(c)
    }
}
impl From<oneshot::Sender<protocol::LatestTipsResponse>> for AnySender {
    fn from(c: oneshot::Sender<protocol::LatestTipsResponse>) -> Self {
        AnySender::LatestTips(c)
    }
}

type PostT = (protocol::Post, oneshot::Sender<protocol::PostResponse>);

#[derive(Clone, Debug)]
pub struct ProtocolChan {
    latest_tips: UnboundedSender<LatestTipsT>,
    specific_tips: UnboundedSender<SpecificTipsT>,
    post: UnboundedSender<PostT>,
}

impl ProtocolChan {
    // if any is closed, they should all be dropped
    pub fn is_closed(&self) -> bool {
        self.post.is_closed() || self.specific_tips.is_closed() || self.latest_tips.is_closed()
    }
    pub fn send_latest_tips(&self, value: LatestTipsT) -> Result<(), SendError<LatestTipsT>> {
        self.latest_tips.send(value)
    }
    pub fn send_specific_tips(&self, value: SpecificTipsT) -> Result<(), SendError<SpecificTipsT>> {
        self.specific_tips.send(value)
    }
    pub fn send_post(&self, value: PostT) -> Result<(), SendError<PostT>> {
        self.post.send(value)
    }
}

pub struct ProtocolReceiverMut<'a> {
    pub latest_tips: &'a mut UnboundedReceiver<LatestTipsT>,
    pub specific_tips: &'a mut UnboundedReceiver<SpecificTipsT>,
    pub post: &'a mut UnboundedReceiver<PostT>,
}
impl Drop for ProtocolReceiver {
    fn drop(&mut self) {
        warn!("Dropping Protocol Receiver");
    }
}
pub struct ProtocolReceiver {
    pub latest_tips: UnboundedReceiver<LatestTipsT>,
    pub specific_tips: UnboundedReceiver<SpecificTipsT>,
    pub post: UnboundedReceiver<PostT>,
}

impl ProtocolReceiver {
    pub fn get_mut(&mut self) -> ProtocolReceiverMut {
        ProtocolReceiverMut {
            latest_tips: &mut self.latest_tips,
            specific_tips: &mut self.specific_tips,
            post: &mut self.post,
        }
    }
}

pub fn new_protocol_chan(_p: usize) -> (ProtocolChan, ProtocolReceiver) {
    let (latest_tips_tx, latest_tips_rx) = unbounded_channel();
    let (specific_tips_tx, specific_tips_rx) = unbounded_channel();
    let (post_tx, post_rx) = unbounded_channel();
    (
        ProtocolChan {
            latest_tips: latest_tips_tx,
            specific_tips: specific_tips_tx,
            post: post_tx,
        },
        ProtocolReceiver {
            latest_tips: latest_tips_rx,
            specific_tips: specific_tips_rx,
            post: post_rx,
        },
    )
}

mod connection_creation;
// Methods
mod http_methods;
mod ws_methods;

static PENDING_COOKIE: AtomicU64 = AtomicU64::new(0);
#[derive(Debug)]
pub enum PeerState {
    Open(ProtocolChan, Role),
    Closed,
    Pending(u64),
}

#[derive(Clone)]
pub struct AttestationClient {
    client: Client,
    inflight: Arc<Mutex<BTreeSet<CanonicalEnvelopeHash>>>,
    connections: Arc<RwLock<HashMap<ServiceUrl, PeerState>>>,
    g: Arc<Globals>,
    db: MsgDB,
    gss: GlobalSocketState,
}

impl AttestationClient {
    pub fn client(&self) -> &Client {
        &self.client
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Serialize, Deserialize, Ord, PartialOrd)]
pub struct ServiceUrl(pub Arc<String>, pub u16);

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
    Already(ProtocolChan),
    Newly(ProtocolChan, ProtocolReceiver),
    Abort,
}
