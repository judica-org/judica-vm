use super::super::query::Tips;
use super::generic_websocket::WebSocketFunctionality;
use super::GlobalSocketState;
use crate::control::query::Outcome;
use attest_database::connection::MsgDB;
use attest_messages::Envelope;
use axum::extract::ws::Message;
use sapio_bitcoin::secp256k1::Secp256k1;
use serde::Deserialize;
use serde::Serialize;
use std;
use std::collections::BTreeMap;
use std::fmt::Display;
use tracing;
use tracing::info;
use tracing::trace;
use tracing::warn;

#[derive(Serialize, Deserialize, Debug)]
pub enum AttestSocketProtocol {
    Request(u64, AttestRequest),
    Response(u64, AttestResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthenticationCookie {
    pub(crate) secret: [u8; 32],
    pub(crate) service_claim: (String, u16),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AttestRequest {
    LatestTips,
    SpecificTips(Tips),
    Post(Vec<Envelope>),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AttestResponse {
    LatestTips(Vec<Envelope>),
    SpecificTips(Vec<Envelope>),
    PostResult(Vec<Outcome>),
}

#[derive(PartialEq, Eq)]
pub struct ResponseCode(u64);

impl AttestResponse {
    pub(crate) fn response_code_of(&self) -> ResponseCode {
        ResponseCode(match self {
            AttestResponse::LatestTips(_) => 0,
            AttestResponse::SpecificTips(_) => 1,
            AttestResponse::PostResult(_) => 2,
        })
    }
    pub(crate) fn to_protocol_and_log(self, seq: u64) -> Result<Message, serde_json::Error> {
        let msg = &AttestSocketProtocol::Response(seq, self);
        trace!(?msg, seq, "Sending Response");
        Ok(Message::Binary(serde_json::to_vec(msg)?))
    }
}

#[derive(Debug)]
pub enum AttestProtocolError {
    JsonError(String),
}

unsafe impl Send for AttestProtocolError {}

unsafe impl Sync for AttestProtocolError {}

impl Display for AttestProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<serde_json::Error> for AttestProtocolError {
    fn from(e: serde_json::Error) -> Self {
        AttestProtocolError::JsonError(e.to_string())
    }
}

impl std::error::Error for AttestProtocolError {}

pub async fn run_protocol<W: WebSocketFunctionality>(
    mut socket: W,
    g: GlobalSocketState,
    db: MsgDB,
) -> Result<(), AttestProtocolError> {
    let inflight_requests: BTreeMap<u64, ResponseCode> = Default::default();

    while let Some(msg) = socket.t_recv().await {
        match msg {
            Ok(m) => match m {
                Message::Text(t) => break,
                Message::Binary(b) => {
                    let a: AttestSocketProtocol = serde_json::from_slice(&b[..])?;
                    match a {
                        AttestSocketProtocol::Request(seq, m) => match m {
                            AttestRequest::LatestTips => {
                                let r = {
                                    let handle = db.get_handle().await;
                                    info!(method = "WS Latest Tips");
                                    handle.get_tips_for_all_users()
                                };
                                if let Ok(v) = r {
                                    if socket
                                        .t_send(
                                            AttestResponse::LatestTips(v)
                                                .to_protocol_and_log(seq)?,
                                        )
                                        .await
                                        .is_err()
                                    {
                                        break;
                                    }
                                } else {
                                    warn!("Database Error, Disconnecting");
                                    break;
                                }
                            }
                            AttestRequest::SpecificTips(mut tips) => {
                                // runs in O(N) usually since the slice should already be sorted
                                tips.tips.sort_unstable();
                                tips.tips.dedup();
                                trace!(method = "GET /tips", ?tips);
                                let all_tips = {
                                    let handle = db.get_handle().await;
                                    if let Ok(r) = handle.messages_by_hash(tips.tips.iter()) {
                                        r
                                    } else {
                                        break;
                                    }
                                };

                                if socket
                                    .t_send(
                                        AttestResponse::SpecificTips(all_tips)
                                            .to_protocol_and_log(seq)?,
                                    )
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            AttestRequest::Post(envelopes) => {
                                let mut authed = Vec::with_capacity(envelopes.len());
                                for envelope in envelopes {
                                    tracing::info!(method="POST /msg",  envelope=?envelope.canonicalized_hash_ref(), "Envelope Received" );
                                    tracing::trace!(method="POST /msg",  envelope=?envelope, "Envelope Received" );
                                    if let Ok(valid_envelope) =
                                        envelope.self_authenticate(&Secp256k1::new())
                                    {
                                        authed.push(valid_envelope);
                                    } else {
                                        tracing::debug!("Invalid Message From Peer");
                                        break;
                                    }
                                }
                                let mut outcomes = Vec::with_capacity(authed.len());
                                {
                                    let mut locked = db.get_handle().await;
                                    for envelope in authed {
                                        tracing::trace!("Inserting Into Database");
                                        match locked.try_insert_authenticated_envelope(envelope) {
                                            Ok(i) => match i {
                                                Ok(()) => {
                                                    outcomes.push(Outcome { success: true });
                                                }
                                                Err(fail) => {
                                                    outcomes.push(Outcome { success: false });
                                                    tracing::debug!(
                                                        ?fail,
                                                        "Inserting Into Database Failed"
                                                    );
                                                }
                                            },
                                            Err(err) => {
                                                outcomes.push(Outcome { success: false });
                                                tracing::debug!(
                                                    ?err,
                                                    "Inserting Into Database Failed"
                                                );
                                            }
                                        }
                                    }
                                }
                                if socket
                                    .t_send(
                                        AttestResponse::PostResult(outcomes)
                                            .to_protocol_and_log(seq)?,
                                    )
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        },
                        AttestSocketProtocol::Response(seq, r) => {
                            if let Some(k) = inflight_requests.get(&seq) {
                                if r.response_code_of() != *k {
                                    break;
                                }
                                match r {
                                    AttestResponse::LatestTips(tips) => todo!(),
                                    AttestResponse::SpecificTips(tips) => todo!(),
                                    AttestResponse::PostResult(outcomes) => todo!(),
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }
                Message::Ping(p) | Message::Pong(p) => {}
                Message::Close(c) => break,
            },
            Err(e) => break,
        }
    }
    socket.t_close().await;
    Ok(())
}
