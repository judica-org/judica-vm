use super::super::generic_websocket::WebSocketFunctionality;
use super::AttestProtocolError;
use super::GlobalSocketState;
use super::ServiceIDBuilder;
use crate::attestations::client::OpenState;
use crate::attestations::client::ProtocolReceiver;
use crate::attestations::client::ServiceUrl;
use crate::globals::Globals;
use axum::extract::ws::Message;
use bitcoincore_rpc_async::bitcoin::hashes::hex::FromHex;
use bitcoincore_rpc_async::bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::hashes::sha256;
use sapio_bitcoin::hashes::Hash;
use sapio_bitcoin::secp256k1::rand;
use sapio_bitcoin::secp256k1::rand::Rng;
use tracing::debug;
use std::sync::Arc;
use std::time::Duration;

use tokio_tungstenite::tungstenite::protocol::Role;
use tracing::{trace, warn};

fn new_cookie() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let challenge_secret: [u8; 32] = rng.gen();
    challenge_secret
}
pub trait MessageExt {
    fn only_text(self, s: &str) -> Result<String, AttestProtocolError>;
}
impl MessageExt for Message {
    fn only_text(self, s: &str) -> Result<String, AttestProtocolError> {
        match self {
            Message::Text(s) => Ok(s),
            Message::Binary(_) => Err(AttestProtocolError::IncorrectMessageOwned(format!(
                "Incorrect Message Type Binary, expected Text {}",
                s
            ))),
            Message::Ping(_) => Err(AttestProtocolError::IncorrectMessageOwned(format!(
                "Incorrect Message Type Ping, expected Text {}",
                s
            ))),
            Message::Pong(_) => Err(AttestProtocolError::IncorrectMessageOwned(format!(
                "Incorrect Message Type Pong, expected Text {}",
                s
            ))),
            Message::Close(_) => Err(AttestProtocolError::IncorrectMessageOwned(format!(
                "Incorrect Message Type Close, expected Text {}",
                s
            ))),
        }
    }
}
pub async fn handshake_protocol_server<W: WebSocketFunctionality>(
    g: Arc<Globals>,
    socket: &mut W,
    _gss: &mut GlobalSocketState,
) -> Result<ServiceUrl, AttestProtocolError> {
    let protocol = "handshake";
    let t = socket
        .t_recv()
        .await
        .ok_or(AttestProtocolError::SocketClosed)??
        .only_text("Expected Text Message to initiate protocol")?;
    {
        let s: ServiceIDBuilder = serde_json::from_str(&t)?;
        let s = ServiceUrl(Arc::new(s.0), s.1);
        let challenge_secret = new_cookie();
        let client = g.get_client().await?;
        let challenge_hash = sha256::Hash::hash(&challenge_secret[..]);
        socket
            .t_send(Message::Text(challenge_hash.to_hex()))
            .await
            .map_err(|_e| AttestProtocolError::SocketClosed)?;
        trace!(protocol, role=?Role::Server, "Challenge Sent, awaiting Acknowledgement");
        let challenge_ack = socket
            .t_recv()
            .await
            .ok_or(AttestProtocolError::SocketClosed)??
            .only_text("for challenge ack")?;
        if !challenge_ack.is_empty() {
            trace!(protocol, role=?Role::Server, "Challenge Rejected (non zero ack)");
            return Err(AttestProtocolError::NonZeroSync);
        }
        trace!(protocol, role=?Role::Server, "Challenge Acknowledged");
        // Ready to go!

        trace!(protocol, role=?Role::Server, "Sending Secret");
        client
            .authenticate(&challenge_secret, &s.0, s.1)
            .await
            .map_err(|_| AttestProtocolError::FailedToAuthenticate)?;
        tokio::time::timeout(Duration::from_secs(10), socket.t_recv())
            .await
            .map_err(|_| AttestProtocolError::TimedOut)?
            .ok_or(AttestProtocolError::SocketClosed)??
            .only_text("for challenge_response")
            .and_then(|c| {
                if c == challenge_secret.to_hex() {
                    Ok(())
                } else {
                    Err(AttestProtocolError::CookieMissMatch)
                }
            })?;
        Ok(s)
    }
}

pub async fn handshake_protocol_client<W: WebSocketFunctionality>(
    g: Arc<Globals>,
    socket: &mut W,
    gss: &mut GlobalSocketState,
) -> Result<(), AttestProtocolError> {
    let me = if let Some(conf) = g.config.tor.as_ref().map(|conf| conf.get_hostname()) {
        conf.await
            .map_err(|_| AttestProtocolError::HostnameUnknown)?
    } else {
        ("127.0.0.1".into(), g.config.attestation_port)
    };

    let protocol = "handshake";
    trace!(protocol, role=?Role::Client, ?me, "Identifying Self to Peer");
    socket
        .t_send(Message::Text(serde_json::to_string(&me)?))
        .await
        .map_err(|_| AttestProtocolError::SocketClosed)?;
    let challenge_hash_string = socket
        .t_recv()
        .await
        .ok_or(AttestProtocolError::SocketClosed)??
        .only_text("for challenge hash type")?;
    trace!(protocol, role=?Role::Client, ?me, "Claimed Identity of Self to Peer");
    let challenge_hash = sha256::Hash::from_hex(&challenge_hash_string)
        .map_err(|_| AttestProtocolError::InvalidChallengeHashString)?;
    trace!(protocol, ?challenge_hash, role=?Role::Client, ?me, "Recieved Challenge");
    let expect = gss.expect_a_cookie(challenge_hash).await;
    socket.t_send(Message::Text("".into())).await.map_err(|_| {
        trace!(protocol, role=?Role::Client, ?me, "Failed to confirm receipt of Challenge");
        AttestProtocolError::TimedOut
    })?;
    trace!(protocol, role=?Role::Client, ?me, "Confirmed Receipt of Challenge");
    trace!(protocol, role=?Role::Client, ?me, "Waiting to Learn Secret");
    let cookie = tokio::time::timeout(Duration::from_secs(10), expect)
        .await
        .map_err(|_| {
            trace!(protocol, role=?Role::Client, ?me, "Timed Out Learning Cookie");
            AttestProtocolError::TimedOut
        })?
        .map_err(|_| {
            trace!(protocol, role=?Role::Client, ?me, "Cookie Channel Dropped");
            AttestProtocolError::TimedOut
        })?;

    trace!(protocol, role=?Role::Client, ?me, "Secret Learned");
    trace!(protocol, role=?Role::Client, "Sending Cookie to Server");
    socket
        .t_send(Message::Text(cookie.to_hex()))
        .await
        .map_err(|_| AttestProtocolError::SocketClosed)?;
    Ok(())
}

pub async fn handshake_protocol<W: WebSocketFunctionality>(
    g: Arc<Globals>,
    socket: &mut W,
    gss: &mut GlobalSocketState,
    role: Role,
) -> Result<Option<ServiceUrl>, AttestProtocolError> {
    trace!(protocol = "handshake", ?role, "Starting Handshake");
    let res = match role {
        (Role::Server) => handshake_protocol_server(g, socket, gss).await.map(Some),
        (Role::Client) => handshake_protocol_client(g, socket, gss)
            .await
            .map(|()| None),
    };

    if let Err(e) = &res {
        debug!(protocol="handshake", error=?e, ?role, "Handshake Protocol Failed");
    } else {
        trace!(protocol = "handshake", ?role, "Handshake Successful");
    }
    res
}
