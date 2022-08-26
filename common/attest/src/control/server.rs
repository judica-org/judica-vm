use crate::{
    peer_services::{PeerQuery, PeerType},
    Config,
};
use attest_database::{connection::MsgDB, db_handle::get::PeerInfo, generate_new_user};
use attest_messages::{CanonicalEnvelopeHash, Envelope};
use attest_util::{AbstractResult, INFER_UNIT};
use axum::{
    http::Response,
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use bitcoin_header_checkpoints::BitcoinCheckPointCache;
use reqwest::Method;
use sapio_bitcoin::{
    secp256k1::{All, Secp256k1},
    KeyPair, XOnlyPublicKey,
};
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{mpsc::Sender, oneshot};
use tower_http::cors::{Any, CorsLayer};

use super::query::{PushMsg, Subscribe, Outcome};

#[derive(Serialize, Deserialize)]
pub struct TipData {
    envelope: Envelope,
    hash: CanonicalEnvelopeHash,
}
#[derive(Serialize, Deserialize)]
pub struct Status {
    peers: Vec<PeerInfo>,
    tips: Vec<TipData>,
    peer_connections: Vec<(String, u16, PeerType)>,
    all_users: Vec<(XOnlyPublicKey, String, bool)>,
}
async fn get_status(
    db: Extension<MsgDB>,
    peer_status: Extension<Sender<PeerQuery>>,
) -> Result<(Response<()>, Json<Status>), (StatusCode, String)> {
    let (tips, peers, all_users) = {
        let handle = db.0.get_handle().await;
        let peers = handle
            .get_all_hidden_services()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let tips = handle
            .get_tips_for_all_users()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let tips = tips
            .into_iter()
            .map(|t| TipData {
                hash: t.canonicalized_hash_ref().unwrap(),
                envelope: t,
            })
            .collect();
        let users = handle.get_all_users().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("User List query failed: {}", e),
            )
        })?;
        let known_keys = handle.get_keymap().map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("KeyMap query failed: {}", e),
            )
        })?;
        let all_users: Vec<_> = users
            .into_iter()
            .map(|(k, v)| (k, v, known_keys.contains_key(&k)))
            .collect();
        (tips, peers, all_users)
    };
    let (tx, rx) = oneshot::channel();
    peer_status
        .send(PeerQuery::RunningTasks(tx))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let peer_connections = rx
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let status = Status {
        peers,
        tips,
        peer_connections,
        all_users,
    };

    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(status),
    ))
}

async fn listen_to_service(
    db: Extension<MsgDB>,
    Json(subscribe): Json<Subscribe>,
) -> Result<(Response<()>, Json<Outcome>), (StatusCode, String)> {
    let _r =
        db.0.get_handle()
            .await
            .upsert_hidden_service(subscribe.url, subscribe.port, Some(true), Some(true))
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(Outcome{success:true}),
    ))
}

async fn push_message_dangerous(
    db: Extension<MsgDB>,
    secp: Extension<Secp256k1<All>>,
    bitcoin_tipcache: Extension<Arc<BitcoinCheckPointCache>>,
    Json(PushMsg { msg, key }): Json<PushMsg>,
) -> Result<(Response<()>, Json<Outcome>), (StatusCode, String)> {
    let handle = db.0.get_handle().await;
    let keys = handle.get_keymap().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("KeyMap failed: {}", e),
        )
    })?;
    let kp = keys
        .get(&key)
        .map(|k| KeyPair::from_secret_key(&secp, k))
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Unknown Key".into()))?;
    let tips = bitcoin_tipcache.0.read_cache().await;
    let env = handle
        .wrap_message_in_envelope_for_user_by_key(msg, &kp, &secp.0, Some(tips), None)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Wrapping Message failed: {}", e),
            )
        })?
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Signing Message failed: {}", e),
            )
        })?;
    handle
        .try_insert_authenticated_envelope(env.self_authenticate(&secp.0).unwrap())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Inserting Message failed: {}", e),
            )
        })?;
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(Outcome{success:true}),
    ))
}
async fn make_genesis(
    db: Extension<MsgDB>,
    secp: Extension<Secp256k1<All>>,
    Json(nickname): Json<String>,
) -> Result<(Response<()>, Json<Envelope>), (StatusCode, String)> {
    let (kp, pre, genesis) = generate_new_user(&secp.0).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Creating Genesis Message failed: {}", e),
        )
    })?;
    let handle = db.0.get_handle().await;
    handle
        .save_keypair(kp)
        .and_then(|()| handle.save_nonce_for_user_by_key(pre, &secp.0, kp.x_only_public_key().0))
        .and_then(|_| {
            handle.insert_user_by_genesis_envelope(
                nickname,
                genesis.self_authenticate(&secp.0).unwrap(),
            )
        })
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Creating Genesis Message failed: {}", e),
            )
        })?;

    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(genesis),
    ))
}
pub async fn run(
    config: Arc<Config>,
    db: MsgDB,
    peer_status: Sender<PeerQuery>,
    bitcoin_tipcache: Arc<BitcoinCheckPointCache>,
) -> tokio::task::JoinHandle<AbstractResult<()>> {
    return tokio::spawn(async move {
        // build our application with a route
        let app = Router::new()
            // `POST /msg` goes to `msg`
            .route(
                "/status",
                get(get_status).layer(
                    CorsLayer::new()
                        .allow_methods([Method::GET, Method::OPTIONS])
                        .allow_headers([
                            reqwest::header::ACCESS_CONTROL_ALLOW_HEADERS,
                            reqwest::header::CONTENT_TYPE,
                        ])
                        .allow_origin(Any),
                ),
            )
            .route(
                "/service",
                post(listen_to_service).layer(
                    CorsLayer::new()
                        .allow_methods([Method::POST, Method::OPTIONS])
                        .allow_headers([
                            reqwest::header::ACCESS_CONTROL_ALLOW_HEADERS,
                            reqwest::header::CONTENT_TYPE,
                        ])
                        .allow_origin(Any),
                ),
            )
            .route(
                "/push_message_dangerous",
                post(push_message_dangerous).layer(
                    CorsLayer::new()
                        .allow_methods([Method::POST, Method::OPTIONS])
                        .allow_headers([
                            reqwest::header::ACCESS_CONTROL_ALLOW_HEADERS,
                            reqwest::header::CONTENT_TYPE,
                        ])
                        .allow_origin(Any),
                ),
            )
            .route(
                "/make_genesis",
                post(make_genesis).layer(
                    CorsLayer::new()
                        .allow_methods([Method::POST, Method::OPTIONS])
                        .allow_headers([
                            reqwest::header::ACCESS_CONTROL_ALLOW_HEADERS,
                            reqwest::header::CONTENT_TYPE,
                        ])
                        .allow_origin(Any),
                ),
            )
            .layer(Extension(db))
            .layer(Extension(peer_status))
            .layer(Extension(Secp256k1::new()))
            .layer(Extension(bitcoin_tipcache))
            .layer(tower_http::trace::TraceLayer::new_for_http());

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        let addr = SocketAddr::from(([127, 0, 0, 1], config.control.port));
        tracing::debug!("Control Service Listening on {}", addr);
        let r = axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await;
        tracing::debug!("Control Service Failed");
        r?;
        INFER_UNIT
    });
}
