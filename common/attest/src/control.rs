use crate::Config;
use attest_database::connection::MsgDB;
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
use tower_http::cors::{Any, CorsLayer};

#[derive(Serialize, Deserialize)]
pub struct Status {
    peers: Vec<(String, u16)>,
}
async fn get_status(
    db: Extension<MsgDB>,
) -> Result<(Response<()>, Json<Status>), (StatusCode, String)> {
    let r =
        db.0.get_handle()
            .await
            .get_all_hidden_services()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let status = Status { peers: r };
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
            .insert_hidden_service(subscribe.url, subscribe.port)
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
pub async fn run(config: Arc<Config>, db: MsgDB) -> tokio::task::JoinHandle<AbstractResult<()>> {
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
            .layer(Extension(db));

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
