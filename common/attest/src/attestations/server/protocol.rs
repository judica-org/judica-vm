use super::super::query::Tips;
use super::generic_websocket::WebSocketFunctionality;
use crate::control::query::Outcome;
use crate::globals::Globals;
use attest_database::connection::MsgDB;
use attest_messages::Envelope;
use axum::extract::ws::Message;
use sapio_bitcoin::hashes::sha256;
use sapio_bitcoin::hashes::Hash;
use sapio_bitcoin::secp256k1::rand;
use sapio_bitcoin::secp256k1::rand::Rng;
use sapio_bitcoin::secp256k1::Secp256k1;
use serde::Deserialize;
use serde::Serialize;
use std;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::oneshot;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Role;
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
    ReqwetError(String),
    HostnameUnknown,
    NonZeroSync,
    IncorrectMessage,
    CookieMissMatch,
    TimedOut,
    AlreadyRunning,
    SocketClosed,
    FailedToAuthenticate,
}

unsafe impl Send for AttestProtocolError {}

unsafe impl Sync for AttestProtocolError {}

impl Display for AttestProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<reqwest::Error> for AttestProtocolError {
    fn from(e: reqwest::Error) -> Self {
        AttestProtocolError::ReqwetError(e.to_string())
    }
}
impl From<serde_json::Error> for AttestProtocolError {
    fn from(e: serde_json::Error) -> Self {
        AttestProtocolError::JsonError(e.to_string())
    }
}

impl std::error::Error for AttestProtocolError {}

type ServiceID = (String, u16);
type ServiceState = Arc<Service>;
type ServiceDB = Arc<Mutex<HashMap<ServiceID, ServiceState>>>;
struct Service {
    is_running: AtomicBool,
}

impl Service {
    fn already_running(&self) -> bool {
        self.is_running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
    }
}

#[derive(Clone, Default)]
pub struct GlobalSocketState {
    services: ServiceDB,
    cookies: Arc<Mutex<BTreeMap<sha256::Hash, (oneshot::Sender<[u8; 32]>, i64)>>>,
}

impl GlobalSocketState {
    pub async fn expect_a_cookie(&self, cookie: sha256::Hash) -> oneshot::Receiver<[u8; 32]> {
        let mut cookiejar = self.cookies.lock().await;
        if cookiejar.len() > 100 {
            let stale = attest_util::now() - 1000 * 20;
            cookiejar.retain(|k, x| x.1 > stale);
            if cookiejar.len() > 100 {
                if let Some(k) = cookiejar.keys().cloned().next() {
                    cookiejar.remove(&k);
                }
            }
        }
        let (tx, rx) = oneshot::channel();

        let e = cookiejar
            .entry(cookie)
            .or_insert_with(|| (tx, attest_util::now()));
        rx
    }
    pub async fn add_a_cookie(&self, cookie: [u8; 32]) {
        let mut cookiejar = self.cookies.lock().await;
        if cookiejar.len() > 100 {
            let stale = attest_util::now() - 1000 * 20;
            cookiejar.retain(|k, x| x.1 > stale);
            if cookiejar.len() > 100 {
                if let Some(k) = cookiejar.keys().cloned().next() {
                    cookiejar.remove(&k);
                }
            }
        }
        let k = sha256::Hash::hash(&cookie);

        if let Some(f) = cookiejar.remove(&k) {
            f.0.send(cookie).ok();
        }
    }
}

pub async fn handshake_protocol_server<W: WebSocketFunctionality>(
    g: Arc<Globals>,
    mut socket: W,
    gss: GlobalSocketState,
) -> Result<W, AttestProtocolError> {
    if let Some(Ok(Message::Text(t))) = socket.t_recv().await {
        let s: ServiceID = serde_json::from_str(&t)?;
        let mut services = gss.services.lock().await;
        let svc = services
            .entry(s.clone())
            .or_insert_with(|| {
                Arc::new(Service {
                    is_running: false.into(),
                })
            })
            .clone();
        drop(services);
        if svc.already_running() {
            return Err(AttestProtocolError::AlreadyRunning);
        }
        let r = new_cookie();
        let client = g.get_client().await?;
        let h = sha256::Hash::hash(&r[..]);
        socket
            .t_send(Message::Binary(h.into_inner().into()))
            .await
            .map_err(|e| AttestProtocolError::SocketClosed)?;

        if let Message::Binary(v) = socket
            .t_recv()
            .await
            .ok_or(AttestProtocolError::SocketClosed)?
            .map_err(|_| AttestProtocolError::SocketClosed)?
        {
            if !v.is_empty() {
                return Err(AttestProtocolError::NonZeroSync);
            }
            // Ready to go!
        } else {
            return Err(AttestProtocolError::IncorrectMessage);
        }

        client
            .authenticate(&r, &s.0, s.1)
            .await
            .map_err(|_| AttestProtocolError::FailedToAuthenticate)?;
        if let Ok(Some(Ok(msg))) =
            tokio::time::timeout(Duration::from_secs(10), socket.t_recv()).await
        {
            if let Message::Binary(v) = msg {
                if v[..] == r {
                    // Authenticated!
                    Ok(socket)
                } else {
                    Err(AttestProtocolError::CookieMissMatch)
                }
            } else {
                Err(AttestProtocolError::IncorrectMessage)
            }
        } else {
            Err(AttestProtocolError::TimedOut)
        }
    } else {
        Err(AttestProtocolError::IncorrectMessage)
    }
}

pub async fn handshake_protocol_client<W: WebSocketFunctionality>(
    g: Arc<Globals>,
    mut socket: W,
    gss: GlobalSocketState,
) -> Result<W, AttestProtocolError> {
    let me = if let Some(conf) = g.config.tor.as_ref() {
        let h = conf
            .get_hostname()
            .await
            .map_err(|_| AttestProtocolError::HostnameUnknown)?;
        h
    } else {
        ("127.0.0.1".into(), g.config.attestation_port)
    };
    if socket
        .t_send(Message::Text(serde_json::to_string(&me)?))
        .await
        .is_err()
    {
        socket.t_close().await.ok();
        Err(AttestProtocolError::SocketClosed)
    } else if let Some(Ok(Message::Binary(v))) = socket.t_recv().await {
        if v.len() == 32 {
            let cookie: [u8; 32] = v.try_into().unwrap();
            let h = sha256::Hash::from_inner(cookie);
            let expect = gss.expect_a_cookie(h).await;
            socket.t_send(Message::Binary(vec![]));
            let cookie = expect.await;
            if let Ok(cookie) = cookie {
                if socket.t_send(Message::Binary(cookie.into())).await.is_err() {
                    Err(AttestProtocolError::SocketClosed)
                } else {
                    Ok(socket)
                }
            } else {
                Err(AttestProtocolError::TimedOut)
            }
        } else {
            Err(AttestProtocolError::IncorrectMessage)
        }
    } else {
        Err(AttestProtocolError::IncorrectMessage)
    }
}
pub async fn handshake_protocol<W: WebSocketFunctionality>(
    g: Arc<Globals>,
    socket: W,
    gss: GlobalSocketState,
    role: Role,
) -> Result<W, AttestProtocolError> {
    match role {
        Role::Server => handshake_protocol_server(g, socket, gss).await,
        Role::Client => handshake_protocol_client(g, socket, gss).await,
    }
}

pub async fn run_protocol<W: WebSocketFunctionality>(
    g: Arc<Globals>,
    socket: W,
    gss: GlobalSocketState,
    db: MsgDB,
    role: Role,
) -> Result<(), AttestProtocolError> {
    let mut socket = handshake_protocol(g, socket, gss, role).await?;
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

fn new_cookie() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let r: [u8; 32] = rng.gen();
    drop(rng);
    r
}
