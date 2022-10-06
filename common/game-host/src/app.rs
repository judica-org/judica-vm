use crate::{
    app::routes::game_init::{add_player, create_new_game_instance, finish_setup, NewGameDB},
    Config,
};
use attest_database::{connection::MsgDB, db_handle::get::PeerInfo, generate_new_user};
use attest_messages::CanonicalEnvelopeHash;
use axum::{
    http::StatusCode,
    http::{
        header::{ACCESS_CONTROL_ALLOW_HEADERS, CONTENT_TYPE},
        Method, Response,
    },
    routing::{get, post},
    Extension, Json, Router,
};
use game_host_messages::{BroadcastByHost, Channelized, Peer};
use mine_with_friends_board::game::GameSetup;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::secp256k1::{All, Secp256k1};
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use std::{error::Error, net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
mod routes;

#[derive(Deserialize, Serialize)]
pub struct Tips {
    pub tips: Vec<CanonicalEnvelopeHash>,
}
pub async fn get_peers(
    Extension(db): Extension<MsgDB>,
) -> Result<(Response<()>, Json<Vec<Peer>>), (StatusCode, &'static str)> {
    let handle = db.get_handle().await;
    let services = handle
        .get_all_hidden_services()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?
        .into_iter()
        .map(
            |PeerInfo {
                 service_url,
                 port,
                 fetch_from: _,
                 push_to: _,
                 allow_unsolicited_tips: _,
             }| Peer { service_url, port },
        )
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
pub async fn add_new_peer(
    Extension(db): Extension<MsgDB>,
    Json(peer): Json<Peer>,
) -> Result<(Response<()>, Json<Value>), (StatusCode, &'static str)> {
    tracing::debug!("Adding Peer: {:?}", peer);
    {
        tracing::debug!("Inserting Into Database");
        let locked = db.get_handle().await;
        locked
            .upsert_hidden_service(
                peer.service_url,
                peer.port,
                Some(true),
                Some(true),
                Some(true),
            )
            .ok();
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

#[derive(Serialize)]
pub struct CreatedNewChain {
    pub genesis_hash: CanonicalEnvelopeHash,
    pub group_name: String,
}

fn flip<T, E1, E2>(r: Result<Result<T, E1>, E2>) -> Result<Result<T, E2>, E1> {
    match r {
        Ok(v) => match v {
            Ok(t) => Ok(Ok(t)),
            Err(e) => Err(e),
        },
        Err(e) => Ok(Err(e)),
    }
}
trait Apply {
    fn apply<F, T>(self, f: F) -> T
    where
        F: FnOnce(Self) -> T,
        Self: Sized,
    {
        f(self)
    }
}
impl<T> Apply for T {}

pub async fn create_new_attestation_chain(
    Json((args, setup)): Json<(Vec<CanonicalEnvelopeHash>, GameSetup)>,
    Extension(db): Extension<MsgDB>,
    Extension(ref secp): Extension<Secp256k1<All>>,
) -> Result<(Response<()>, Json<CreatedNewChain>), (StatusCode, &'static str)> {
    tracing::debug!("Creating New Attestation Chain");
    let (kp, n, e) = generate_new_user::<_, Channelized<BroadcastByHost>, _>(
        secp,
        Channelized {
            data: BroadcastByHost::GameSetup(setup),
            channel: "default".into(),
        },
    )
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;
    let e = e
        .self_authenticate(secp)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;
    let genesis_hash = e.get_genesis_hash();
    let nickname = e.get_genesis_hash().to_hex();
    let group_name = {
        let mut handle = db.get_handle().await;
        Ok(())
            .and_then(|_| handle.save_keypair(kp))
            .and_then(|_| handle.save_nonce_for_user_by_key(n, secp, kp.x_only_public_key().0))
            .and_then(|_| handle.insert_user_by_genesis_envelope(nickname, e))
            .apply(flip)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?
            .and_then(|_| handle.new_chain_commit_group(None))
            .and_then(|(name, group_id)| {
                handle.add_subscriber_to_chain_commit_group(group_id, genesis_hash)?;
                for genesis_hash in args {
                    handle.add_member_to_chain_commit_group(group_id, genesis_hash)?;
                }
                Ok(name)
            })
    }
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(CreatedNewChain {
            genesis_hash,
            group_name,
        }),
    ))
}

pub async fn list_groups(
    Extension(db): Extension<MsgDB>,
) -> Result<(Response<()>, Json<Vec<String>>), (StatusCode, &'static str)> {
    let handle = db.get_handle().await;
    let groups = handle
        .get_all_chain_commit_groups()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;
    let v = groups.into_iter().map(|g| g.1).collect();
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(v),
    ))
}

#[derive(Deserialize)]
pub struct AddChainToGroup {
    genesis_hash: CanonicalEnvelopeHash,
    group: String,
}
pub async fn add_chain_to_group(
    Json(j): Json<AddChainToGroup>,
    Extension(db): Extension<MsgDB>,
) -> Result<(Response<()>, Json<()>), (StatusCode, &'static str)> {
    let handle = db.get_handle().await;
    // todo: more efficient query
    let groups: Vec<_> = handle
        .get_all_chain_commit_groups()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;
    let id = groups
        .iter()
        .find(|x| x.1 == j.group)
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, ""))?
        .0;
    handle
        .add_member_to_chain_commit_group(id, j.genesis_hash)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(()),
    ))
}

pub fn run(
    config: Arc<Config>,
    db: MsgDB,
) -> tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync + 'static>>> {
    let secp = Secp256k1::new();
    tokio::spawn(async move {
        // build our application with a route
        let app = Router::new()
            // `POST /msg` goes to `msg`
            .route("/peer/new", post(add_new_peer))
            .route("/game/new", post(create_new_game_instance))
            .route("/game/player/new", post(add_player))
            .route("/game/finish", post(finish_setup))
            .route("/peer", get(get_peers))
            .route("/attestation_chain/new", post(create_new_attestation_chain))
            .route("/attestation_chain", get(list_groups))
            .route(
                "/attestation_chain/commit_group/add_member",
                post(add_chain_to_group),
            )
            .layer(Extension(db))
            .layer(Extension(secp))
            .layer(Extension(Arc::new(Mutex::new(NewGameDB::new()))))
            .layer(
                CorsLayer::new()
                    .allow_methods([Method::GET, Method::OPTIONS, Method::POST])
                    .allow_headers([ACCESS_CONTROL_ALLOW_HEADERS, CONTENT_TYPE])
                    .allow_origin(Any),
            );

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        let addr = SocketAddr::from(([127, 0, 0, 1], config.tor.application_port));
        tracing::debug!("listening on {}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;
        Ok(())
    })
}
