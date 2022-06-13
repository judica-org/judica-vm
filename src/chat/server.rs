use axum::{
    http::StatusCode,
    http::{self, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use ruma_serde::Base64;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::hashes::Hash;
use sqlite::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

use crate::chat::messages::InnerMessage;

use super::{
    db::MsgDB,
    messages::{Envelope, MessageResponse},
};
async fn post_message(
    Extension(db): Extension<MsgDB>,
    Json(envelope): Json<Envelope>,
) -> Result<(Response<()>, Json<MessageResponse>), (StatusCode, &'static str)> {
    if envelope.channel.len() > 128 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Channel ID Longer than 128 Characters",
        ));
    }
    tracing::debug!("recieved: {:?}", envelope);
    let userid = {
        let mut pkmap2 = ruma_signatures::PublicKeyMap::new();
        let keyhash = sapio_bitcoin::hashes::sha256::Hash::hash(envelope.key.as_bytes());
        let key = Base64::<ruma_serde::base64::Standard>::new(envelope.key.as_bytes().to_vec());
        let hex_key = keyhash.to_hex();
        pkmap2.insert(
            hex_key.clone(),
            [("ed25519:1".to_owned(), key)].into_iter().collect(),
        );
        let userid = {
            let locked = db.get_handle().await;
            locked
                .locate_user(hex_key)
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?
                .ok_or((StatusCode::BAD_REQUEST, "No User Found"))?
        };

        {
            let reserialized = serde_json::to_value(envelope.clone())
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;
            let signed = serde_json::from_value(reserialized)
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;
            ruma_signatures::verify_json(&pkmap2, &signed)
                .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid Signatures"))?;
            tracing::debug!("verified signatures");
        }
        userid
    };
    let r = match envelope.msg {
        InnerMessage::Ping(data) => Json(MessageResponse::Pong(data)),
        InnerMessage::Data(data) => {
            {
                let locked = db.get_handle().await;
                locked
                    .insert_msg(data, envelope.channel, envelope.sent_time_ms, userid)
                    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;
            }
            Json(MessageResponse::None)
        }
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
