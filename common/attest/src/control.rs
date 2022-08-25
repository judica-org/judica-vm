use crate::{
    peer_services::{PeerQuery, PeerType},
    Config,
};
use attest_database::{connection::MsgDB, db_handle::get::PeerInfo};
use attest_messages::{CanonicalEnvelopeHash, Envelope};
use attest_util::{AbstractResult, INFER_UNIT};
use axum::{
    extract::Query,
    http::Response,
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use reqwest::Method;
use sapio_bitcoin::secp256k1::Secp256k1;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{mpsc::Sender, oneshot};
use tower_http::cors::{Any, CorsLayer};

#[derive(Serialize, Deserialize)]
pub struct Status {
    peers: Vec<PeerInfo>,
    tips: Vec<Envelope>,
    peer_connections: Vec<(String, u16, PeerType)>,
}
async fn get_status(
    db: Extension<MsgDB>,
    peer_status: Extension<Sender<PeerQuery>>,
) -> Result<(Response<()>, Json<Status>), (StatusCode, String)> {
    let handle = db.0.get_handle().await;
    let peers = handle
        .get_all_hidden_services()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let tips = handle
        .get_tips_for_all_users()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (tx, rx) = oneshot::channel();
    peer_status.send(PeerQuery::RunningTasks(tx)).await;
    let peer_connections = rx
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let status = Status {
        peers,
        tips,
        peer_connections,
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

#[derive(Serialize, Deserialize)]
struct Subscribe {
    url: String,
    port: u16,
}
async fn listen_to_service(
    db: Extension<MsgDB>,
    Json(subscribe): Json<Subscribe>,
) -> Result<(Response<()>, Json<Value>), (StatusCode, String)> {
    let r =
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
        Json(json!({"success":true})),
    ))
}
pub async fn run(
    config: Arc<Config>,
    db: MsgDB,
    peer_status: Sender<PeerQuery>,
) -> tokio::task::JoinHandle<AbstractResult<()>> {
    return tokio::spawn(async move {
        // build our application with a route
        let app = Router::new()
            // `POST /msg` goes to `msg`
            .route("/status", get(get_status))
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
            .layer(Extension(db))
            .layer(Extension(peer_status));

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
