use crate::Config;
use attest_database::connection::MsgDB;
use attest_messages::{CanonicalEnvelopeHash, Envelope};
use axum::{
    extract::Query,
    http::Response,
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use game_host_messages::Peer;
use sapio_bitcoin::secp256k1::Secp256k1;
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use std::{error::Error, net::SocketAddr, sync::Arc};

#[derive(Deserialize, Serialize)]
pub struct Tips {
    pub tips: Vec<CanonicalEnvelopeHash>,
}
pub async fn get_users(
    Extension(db): Extension<MsgDB>,
) -> Result<(Response<()>, Json<Vec<Peer>>), (StatusCode, &'static str)> {
    let handle = db.get_handle().await;
    let services = handle
        .get_all_hidden_services()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?
        .into_iter()
        .map(|(tor, port)| Peer { tor, port })
        .collect();
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(services),
    ))
}
pub async fn add_user(
    Extension(db): Extension<MsgDB>,
    Json(peer): Json<Peer>,
) -> Result<(Response<()>, Json<Value>), (StatusCode, &'static str)> {
    tracing::debug!("Adding Peer: {:?}", peer);
    {
        tracing::debug!("Inserting Into Database");
        let locked = db.get_handle().await;
        locked.insert_hidden_service(peer.tor, peer.port).ok();
    }
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(json!("Success")),
    ))
}

pub fn run(
    config: Arc<Config>,
    db: MsgDB,
) -> tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync + 'static>>> {
    return tokio::spawn(async move {
        // build our application with a route
        let app = Router::new()
            // `POST /msg` goes to `msg`
            .route("/user", post(add_user))
            .route("/user", get(get_users))
            .layer(Extension(db));

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        let addr = SocketAddr::from(([127, 0, 0, 1], config.tor.application_port));
        tracing::debug!("listening on {}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;
        Ok(())
    });
}