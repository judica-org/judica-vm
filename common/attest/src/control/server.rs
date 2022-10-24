// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    globals::Globals,
    peer_services::{PeerQuery, TaskID},
};
use attest_database::{
    connection::MsgDB,
    db_handle::{
        create::TipControl,
        get::{nonces::extract_sk_from_envelopes, PeerInfo},
    },
    generate_new_user, generate_new_user_keypair,
};
use attest_messages::{Authenticated, CanonicalEnvelopeHash, Envelope, WrappedJson};
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
    secp256k1::{All, Secp256k1, SecretKey},
    util::bip32::ExtendedPrivKey,
    KeyPair, XOnlyPublicKey,
};
use serde::Deserialize;
use serde::Serialize;

use std::{collections::HashMap, net::SocketAddr, str::FromStr, sync::Arc};
use tokio::{
    sync::{mpsc::Sender, oneshot},
    task::spawn_blocking,
};
use tower_http::cors::{Any, CorsLayer};

use super::query::{NewGenesis, Outcome, PushMsg, Subscribe};

#[derive(Serialize, Deserialize)]
pub struct TipData {
    envelope: Envelope,
    hash: CanonicalEnvelopeHash,
}
#[derive(Serialize, Deserialize)]
pub struct Status {
    peers: Vec<PeerInfo>,
    tips: Vec<TipData>,
    peer_connections: Vec<TaskID>,
    all_users: Vec<(XOnlyPublicKey, String, bool, Option<SecretKey>)>,
    hidden_service_url: Option<(String, u16)>,
}

async fn get_expensive_db_snapshot(
    db: Extension<MsgDB>,
) -> Result<(Response<()>, Json<HashMap<CanonicalEnvelopeHash, Envelope>>), (StatusCode, String)> {
    let handle = db.get_handle_read().await;
    let map = spawn_blocking(move || {
        let mut map = Default::default();
        let mut newer = None;
        handle
            .get_all_messages_collect_into_inconsistent_skip_invalid::<Envelope, WrappedJson>(
                &mut newer, &mut map, false,
            )
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Ok(map)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(map),
    ))
}

#[derive(Serialize)]
struct ChainCommitGroupInfo {
    genesis: CanonicalEnvelopeHash,
    members: Vec<Envelope>,
    all_msgs: HashMap<CanonicalEnvelopeHash, Envelope>,
}
async fn chain_commit_groups(
    Json(key): Json<CanonicalEnvelopeHash>,
    db: Extension<MsgDB>,
) -> Result<(Response<()>, Json<ChainCommitGroupInfo>), (StatusCode, String)> {
    let handle = db.0.get_handle_read().await;
    let resp = spawn_blocking(move || {
            let genesis = &handle.messages_by_hash::<_, Envelope, _>(std::iter::once(&key))?[0];
            let _groups = handle.get_all_chain_commit_groups_for_chain(key)?;
            let group_members = handle.get_all_chain_commit_group_members_for_chain(key)?;
            let group_tips = handle.messages_by_ids::<_, Envelope, _>(group_members.iter())?;
            let mut map = Default::default();
            let mut newer = 0;
            handle
            .get_all_chain_commit_group_members_new_envelopes_for_chain_into_inconsistent::<Envelope, WrappedJson>(
                genesis.header().key(),
                &mut newer,
                &mut map)?;
            Ok::<_, rusqlite::Error>(ChainCommitGroupInfo{
                genesis: key,
                members: group_tips,
                all_msgs: map
            })
        }).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(resp),
    ))
}
async fn get_status(
    g: Extension<Arc<Globals>>,
    db: Extension<MsgDB>,
    peer_status: Extension<Sender<PeerQuery>>,
) -> Result<(Response<()>, Json<Status>), (StatusCode, String)> {
    let (tips, peers, all_users) = {
        let handle = db.0.get_handle_read().await;
        spawn_blocking(move || {
            let peers = handle
                .get_all_hidden_services()
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let tips = handle
                .get_tips_for_all_users::<Authenticated<Envelope>, WrappedJson>()
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let tips = tips
                .into_iter()
                .map(|t| TipData {
                    hash: t.canonicalized_hash_ref(),
                    envelope: t.inner(),
                })
                .collect();
            let users = handle.get_all_users().map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("User List query failed: {}", e),
                )
            })?;
            let equivs = handle
                .get_reused_nonces()
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("KeyMap query failed: {}", e),
                    )
                })?
                .into_iter()
                .flat_map(|(x, mut y)| {
                    Some((x, Some(extract_sk_from_envelopes(y.pop()?, y.pop()?)?)))
                })
                .collect::<HashMap<_, _>>();
            let known_keys = handle.get_keymap().map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("KeyMap query failed: {}", e),
                )
            })?;
            let all_users: Vec<_> = users
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        v,
                        known_keys.contains_key(&k),
                        equivs.get(&k).cloned().flatten(),
                    )
                })
                .collect();
            Ok::<_, (StatusCode, String)>((tips, peers, all_users))
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??
    };
    let (tx, rx) = oneshot::channel();
    peer_status
        .send(PeerQuery::RunningTasks(tx))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let peer_connections = rx
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let hidden_service_url = if let Some(conf) = g.config.tor.as_ref() {
        let h = conf
            .get_hostname()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Some(h)
    } else {
        None
    };
    let status = Status {
        peers,
        tips,
        peer_connections,
        all_users,
        hidden_service_url,
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
    Json(Subscribe {
        url,
        port,
        fetch_from,
        push_to,
        allow_unsolicited_tips,
    }): Json<Subscribe>,
    peer_status: Extension<Sender<PeerQuery>>,
) -> Result<(Response<()>, Json<Outcome>), (StatusCode, String)> {
    let h = db.0.get_handle_all().await;
    spawn_blocking(move || {
        h.upsert_hidden_service(url, port, fetch_from, push_to, allow_unsolicited_tips)
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    peer_status.send(PeerQuery::RefreshTasks).await.ok();
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(Outcome { success: true }),
    ))
}

async fn push_message_dangerous(
    db: Extension<MsgDB>,
    secp: Extension<Secp256k1<All>>,
    bitcoin_tipcache: Extension<Arc<BitcoinCheckPointCache>>,
    Json(PushMsg {
        mut msg,
        key,
        equivocate,
    }): Json<PushMsg>,
) -> Result<(Response<()>, Json<Outcome>), (StatusCode, String)> {
    let mut handle = db.0.get_handle_all().await;
    let tips = bitcoin_tipcache.0.read_cache().await;
    spawn_blocking(move || {
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
        if equivocate {
            match msg {
                ruma_serde::CanonicalJsonValue::Array(mut a) if a.len() == 2 => {
                    let dirty1 = a.pop().unwrap();
                    let dirty2 = a.pop().unwrap();
                    let tip = handle
                        .get_tip_for_user_by_key(kp.x_only_public_key().0)
                        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("No Tip: {}", e)))?
                        .inner();
                    for dirty in [dirty1, dirty2] {
                        let m: Envelope = handle
                            .wrap_message_in_envelope_for_user_by_key(
                                dirty,
                                &kp,
                                &secp,
                                Some(tips.clone()),
                                Some(tip.clone()),
                                TipControl::NoTips,
                            )
                            .map_err(|e| {
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    format!("Wrapping Message and Inserting failed: {}", e),
                                )
                            })?
                            .map_err(|e| {
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    format!("Wrapping Message and Inserting failed: {}", e),
                                )
                            })?;
                        handle.try_insert_authenticated_envelope(
                            m.self_authenticate(&secp).unwrap(),
                            false,
                        );
                    }
                }
                _ => {}
            }
        } else {
            handle
                .retry_insert_authenticated_envelope_atomic::<WrappedJson, _, _>(
                    msg,
                    &kp,
                    &secp.0,
                    Some(tips),
                    TipControl::AllTips,
                )
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Wrapping Message and Inserting failed: {}", e),
                    )
                })?;
        };
        Ok::<_, (StatusCode, String)>(())
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(Outcome { success: true }),
    ))
}
async fn make_genesis(
    db: Extension<MsgDB>,
    secp: Extension<Secp256k1<All>>,
    Json(NewGenesis {
        nickname,
        msg,
        danger_extended_private_key,
    }): Json<NewGenesis>,
) -> Result<(Response<()>, Json<Envelope>), (StatusCode, String)> {
    let (kp, pre, genesis) = if let Some(epk) = danger_extended_private_key {
        let epk = ExtendedPrivKey::from_str(&epk).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid ExtendedPrivateKey".to_string(),
            )
        })?;
        let kp = epk.to_keypair(&secp);
        generate_new_user_keypair::<_, WrappedJson, _>(&secp.0, msg, kp)
    } else {
        generate_new_user::<_, WrappedJson, _>(&secp.0, msg)
    }
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Creating Genesis Message failed: {}", e),
        )
    })?;
    let mut handle = db.0.get_handle_all().await;
    let genesis_cpy = genesis.clone();
    spawn_blocking(move || {
        handle
            .save_keypair(kp)
            .and_then(|()| {
                handle.save_nonce_for_user_by_key(pre, &secp.0, kp.x_only_public_key().0)
            })
            .and_then(|_| {
                handle.insert_user_by_genesis_envelope(
                    nickname,
                    genesis_cpy.self_authenticate(&secp.0).unwrap(),
                )
            })
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Creating Genesis Message failed: {}", e),
                )
            })?
            .expect("Should always succeed at inserting a fresh Genesis");
        Ok::<_, (StatusCode, String)>(())
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))??;

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
    g: Arc<Globals>,
    db: MsgDB,
    peer_status: Sender<PeerQuery>,
    bitcoin_tipcache: Arc<BitcoinCheckPointCache>,
) -> tokio::task::JoinHandle<AbstractResult<()>> {
    tokio::spawn(async move {
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
                "/chain_commit_groups",
                post(chain_commit_groups).layer(
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
                "/expensive_db_snapshot",
                get(get_expensive_db_snapshot).layer(
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
            .layer(Extension(g.clone()))
            .layer(Extension(db))
            .layer(Extension(peer_status))
            .layer(Extension(Secp256k1::new()))
            .layer(Extension(bitcoin_tipcache))
            .layer(tower_http::trace::TraceLayer::new_for_http());

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        let addr = SocketAddr::from(([127, 0, 0, 1], g.config.control.port));
        tracing::debug!("Control Service Listening on {}", addr);
        let r = axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await;
        tracing::debug!("Control Service Failed");
        r?;
        INFER_UNIT
    })
}
