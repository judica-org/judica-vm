use super::query::Tips;
use crate::{control::query::Outcome, Config};
use attest_database::connection::MsgDB;
use attest_messages::Envelope;
use attest_util::{AbstractResult, INFER_UNIT};
use axum::{
    extract::ConnectInfo,
    http::Response,
    http::StatusCode,
    routing::{get, post},
    Extension, Json, Router,
};
use sapio_bitcoin::secp256k1::Secp256k1;

use std::{net::SocketAddr, sync::Arc};
use tower_http::trace::TraceLayer;
use tracing::{debug, info, info_span, trace};

pub async fn get_tip_handler(
    Extension(db): Extension<MsgDB>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(query): Json<Option<Tips>>,
) -> Result<(Response<()>, Json<Vec<Envelope>>), (StatusCode, &'static str)> {
    let handle = db.get_handle().await;
    let qtype = query.is_some();
    trace!(from=?addr, method="GET /tips", ?query);
    info!(from=?addr, method="GET /tips", ?query);
    let r = match query {
        Some(Tips { mut tips }) => {
            // runs in O(N) usually since the slice should already be sorted
            tips.sort_unstable();
            tips.dedup();
            handle.messages_by_hash(tips.iter())
        }
        None => handle.get_tips_for_all_users(),
    }
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;

    trace!(from=?addr, method="GET /tips", req_type = if qtype{"specific"}else{"latest"}, response=?r);

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
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(envelopes): Json<Vec<Envelope>>,
) -> Result<(Response<()>, Json<Outcome>), (StatusCode, &'static str)> {
    let mut authed = Vec::with_capacity(envelopes.len());
    for envelope in envelopes {
        tracing::info!(method="POST /msg", from=?addr, envelope=?envelope.canonicalized_hash_ref().unwrap(), "Envelope Received" );
        tracing::trace!(method="POST /msg", from=?addr, envelope=?envelope, "Envelope Received" );
        let envelope = envelope.self_authenticate(&Secp256k1::new()).map_err(|_| {
            tracing::debug!("Invalid Message From Peer");
            (
                StatusCode::UNAUTHORIZED,
                "Envelope not valid. Only valid data should be sent.",
            )
        })?;
        tracing::trace!("Verified Signatures");
        authed.push(envelope);
    }
    {
        let locked = db.get_handle().await;
        for envelope in authed {
            tracing::trace!("Inserting Into Database");
            locked
                .try_insert_authenticated_envelope(envelope)
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?
                .map_err(|err| {
                    tracing::debug!(?err, "Inserting Into Database Failed");
                    (StatusCode::INTERNAL_SERVER_ERROR, "")
                })?;
        }
    }
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(Outcome { success: true }),
    ))
}

pub async fn run(config: Arc<Config>, db: MsgDB) -> tokio::task::JoinHandle<AbstractResult<()>> {
    return tokio::spawn(async move {
        tracing::debug!("Starting Task for Attestation Server");
        // build our application with a route
        let app = Router::new()
            // `POST /msg` goes to `msg`
            .route("/msg", post(post_message))
            .route("/tips", get(get_tip_handler))
            .layer(Extension(db))
            .layer(TraceLayer::new_for_http());

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        let addr = SocketAddr::from(([127, 0, 0, 1], config.attestation_port));
        tracing::debug!("Attestation Server Listening on {}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .unwrap();
        INFER_UNIT
    });
}
