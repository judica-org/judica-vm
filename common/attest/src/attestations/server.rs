// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use self::protocol::GlobalSocketState;
use crate::globals::Globals;
use attest_database::connection::MsgDB;
use attest_util::{AbstractResult, INFER_UNIT};
use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade},
    http::Response,
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use std::{net::SocketAddr, sync::Arc};
use tokio_tungstenite::tungstenite::protocol::Role;
use tower_http::trace::TraceLayer;
use tracing::trace;
pub mod generic_websocket;
pub mod protocol;
pub mod tungstenite_client_adaptor;

async fn handle_socket(
    ws: WebSocketUpgrade,
    Extension(g): Extension<Arc<Globals>>,
    Extension(gss): Extension<GlobalSocketState>,
    Extension(db): Extension<MsgDB>,
) -> axum::response::Response {
    ws.on_upgrade(|w| handle_socket_symmetric_server(g, w, gss, db))
}
async fn handle_socket_symmetric_server(
    g: Arc<Globals>,
    socket: WebSocket,
    gss: GlobalSocketState,
    db: MsgDB,
) {
    let res = protocol::run_protocol(g, socket, gss, db, Role::Server, None).await;
    trace!(?res, role=?Role::Server,"socket quit");
}
pub async fn handle_authenticate(
    Extension(gss): Extension<GlobalSocketState>,
    Json(cookie): Json<[u8; 32]>,
) -> Result<(Response<()>, Json<()>), (StatusCode, &'static str)> {
    gss.add_a_cookie(cookie).await;
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(()),
    ))
}

pub async fn run(g: Arc<Globals>, db: MsgDB) -> tokio::task::JoinHandle<AbstractResult<()>> {
    tokio::spawn(async move {
        tracing::debug!("Starting Task for Attestation Server");
        // build our application with a route
        let app = Router::new()
            // `POST /msg` goes to `msg`
            .route("/socket", get(handle_socket))
            .route("/authenticate", post(handle_authenticate))
            .layer(Extension(db))
            .layer(Extension(g.clone()))
            .layer(Extension(g.socket_state.clone()))
            .layer(TraceLayer::new_for_http());

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        let addr = SocketAddr::from(([127, 0, 0, 1], g.config.attestation_port));
        tracing::debug!("Attestation Server Listening on {}", addr);
        let s = axum::Server::bind(&addr)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await;
        tracing::warn!("The HTTP Server Quit");
        s.unwrap();
        INFER_UNIT
    })
}
