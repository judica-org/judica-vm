use super::super::generic_websocket::WebSocketFunctionality;
use super::AttestProtocolError;
use super::AttestRequest;
use super::AttestResponse;
use super::GlobalSocketState;
use super::Service;
use super::ServiceID;
use crate::attestations::client::ServiceUrl;
use crate::globals::Globals;
use axum::extract::ws::Message;
use sapio_bitcoin::hashes::sha256;
use sapio_bitcoin::hashes::Hash;
use sapio_bitcoin::secp256k1::rand;
use sapio_bitcoin::secp256k1::rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::oneshot;
use tokio_tungstenite::tungstenite::protocol::Role;
use tracing::{trace, warn};

fn new_cookie() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let r: [u8; 32] = rng.gen();
    drop(rng);
    r
}
pub async fn handshake_protocol_server<W: WebSocketFunctionality>(
    g: Arc<Globals>,
    mut socket: W,
    gss: GlobalSocketState,
) -> Result<(W, InternalRequest), AttestProtocolError> {
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
            .map_err(|_e| AttestProtocolError::SocketClosed)?;

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
                    let (tx, rx) = unbounded_channel();
                    if client
                        .conn_already_exists_or_create(&ServiceUrl(s.0, s.1), tx)
                        .await
                    {
                        Ok((socket, rx))
                    } else {
                        Err(AttestProtocolError::AlreadyConnected)
                    }
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
        conf.get_hostname()
            .await
            .map_err(|_| AttestProtocolError::HostnameUnknown)?
    } else {
        ("127.0.0.1".into(), g.config.attestation_port)
    };
    trace!(?me, "Identifying Self to Peer");
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
            if socket.t_send(Message::Binary(vec![])).await.is_err() {
                trace!("Failed to Confirm Receipt of Challenge");
                return Err(AttestProtocolError::TimedOut);
            }
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

pub(crate) type InternalRequest =
    UnboundedReceiver<(AttestRequest, oneshot::Sender<AttestResponse>)>;

pub async fn handshake_protocol<W: WebSocketFunctionality>(
    g: Arc<Globals>,
    socket: W,
    gss: GlobalSocketState,
    role: Role,
    new_request: Option<InternalRequest>,
) -> Result<(W, InternalRequest), AttestProtocolError> {
    let res = match (role, new_request) {
        (Role::Server, None) => handshake_protocol_server(g, socket, gss).await,
        (Role::Client, Some(r)) => Ok((handshake_protocol_client(g, socket, gss).await?, r)),
        _ => {
            warn!("Invalid Combo of Client/Server Role and Channel");
            Err(AttestProtocolError::InvalidSetup)
        }
    };
    if let Err(e) = &res {
        tracing::trace!(error=?e, ?role, "Handshake Protocol Failed");
    }
    res
}
