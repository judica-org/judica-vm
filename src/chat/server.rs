use axum::{
    http::StatusCode,
    http::{self, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use ruma_serde::{Base64, CanonicalJsonValue};
use sapio_bitcoin::hashes::Hash;
use sapio_bitcoin::{
    hashes::hex::ToHex,
    secp256k1::{Message as SchnorrMessage, Secp256k1, Verification},
};
use std::sync::Arc;
use std::time::SystemTime;
use std::{collections::BTreeMap, env, net::SocketAddr};
use tokio::sync::Mutex;

use crate::chat::messages::InnerMessage;

use super::{
    db::MsgDB,
    messages::{Envelope, MessageResponse},
};

pub async fn get_tip_handler(
    Extension(db): Extension<MsgDB>,
) -> Result<(Response<()>, Json<Vec<Envelope>>), (StatusCode, &'static str)> {
    let r = db
        .get_handle()
        .await
        .get_tip_for_known_keys()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(r),
    ))
}
pub async fn post_message(
    Extension(db): Extension<MsgDB>,
    Json(envelope): Json<Envelope>,
) -> Result<(Response<()>, Json<MessageResponse>), (StatusCode, &'static str)> {
    tracing::debug!("Envelope Received: {:?}", envelope);
    let envelope = envelope
        .self_authenticate(&Secp256k1::new())
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Envelope not valid"))?;
    tracing::debug!("Verified Signatures");
    {
        tracing::debug!("Inserting Into Database");
        let locked = db.get_handle().await;
        locked
            .try_insert_authenticated_envelope(envelope.clone())
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;
    }
    tracing::debug!("Responding");
    let r = match &envelope.inner_ref().msg {
        InnerMessage::Ping(u) => {
            let ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Clock Error"))?
                .as_millis() as u64;
            Json(MessageResponse::Pong(*u, ms))
        }
        InnerMessage::Data(data) => Json(MessageResponse::None),
    };
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        r,
    ))
}

pub async fn run(port: u16, db: MsgDB) -> tokio::task::JoinHandle<()> {
    return tokio::spawn(async move {
        // build our application with a route
        let app = Router::new()
            // `POST /msg` goes to `msg`
            .route("/msg", post(post_message))
            .route("/tips", get(get_tip_handler))
            .layer(Extension(db));

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        tracing::debug!("listening on {}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
}
