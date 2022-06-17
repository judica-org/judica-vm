use axum::{
    extract::Query,
    http::Response,
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};

use sapio_bitcoin::hashes::sha256;
use sapio_bitcoin::secp256k1::Secp256k1;
use serde::Deserialize;
use serde::Serialize;

use std::net::SocketAddr;

use crate::attestations::messages::InnerMessage;

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageResponse {
    Pong(u64, u64),
    None,
}
use super::{
    db::connection::MsgDB,
    messages::{Envelope},
};

#[derive(Deserialize, Serialize)]
pub struct Tips {
    pub tips: Vec<sha256::Hash>,
}
pub async fn get_tip_handler(
    Extension(db): Extension<MsgDB>,
    Query(query): Query<Option<Tips>>,
) -> Result<(Response<()>, Json<Vec<Envelope>>), (StatusCode, &'static str)> {
    let handle = db.get_handle().await;
    let r = match query {
        Some(Tips { mut tips }) => {
            // runs in O(N) usually since the slice should already be sorted
            tips.sort_unstable();
            tips.dedup();
            handle.messages_by_hash(tips.iter())
        }
        None => handle.get_tip_for_known_keys(),
    }
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
        InnerMessage::Data(_data) => Json(MessageResponse::None),
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
