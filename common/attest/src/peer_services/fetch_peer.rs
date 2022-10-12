// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;
use crate::attestations::client::AttestationClient;
use crate::attestations::client::NotifyOnDrop;
use crate::attestations::client::ServiceUrl;
use crate::attestations::query::Tips;
use attest_database::sql_error::SqliteFail;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::Envelope;
use attest_util::now;
use attest_util::INFER_UNIT;

use tokio::sync::mpsc::UnboundedSender;
use tracing::info;
use tracing::trace;
use tracing::warn;

pub(crate) async fn fetch_from_peer(
    g: Arc<Globals>,
    client: AttestationClient,
    service: &ServiceUrl,
    conn: MsgDB,
    allow_unsolicited_tips: bool,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let (request_tips, tips_to_resolve) =
        tokio::sync::mpsc::unbounded_channel::<Vec<CanonicalEnvelopeHash>>();
    let (envelopes_to_process, next_envelope) = tokio::sync::mpsc::unbounded_channel();

    // Spins in a loop getting the latest tips from a peer and emitting to
    // envelopes_to_process
    let mut latest_tip_fetcher = latest_tip_fetcher(
        g.clone(),
        client.clone(),
        service,
        envelopes_to_process.clone(),
    );
    // Reads from next_envelope, processes results, and then requests to resolve unknown tips
    let mut envelope_processor = envelope_processor(
        g.clone(),
        service,
        conn,
        next_envelope,
        request_tips,
        allow_unsolicited_tips,
    );
    // fetches unknown envelopes
    let mut missing_envelope_fetcher = missing_envelope_fetcher(
        g.clone(),
        client.clone(),
        service,
        envelopes_to_process.clone(),
        tips_to_resolve,
    );
    tokio::select! {
        a = &mut envelope_processor => {
            warn!(?service, task="FETCH", subtask="Envelope Processor", event="SHUTDOWN", err=?a);
            latest_tip_fetcher.abort();
            missing_envelope_fetcher.abort();
            a??
        }
        a = &mut latest_tip_fetcher => {
            warn!(?service, task="FETCH", subtask="Latest Tip Fetcher", event="SHUTDOWN", err=?a);
            envelope_processor.abort();
            missing_envelope_fetcher.abort();
            a??
        }
        a = &mut missing_envelope_fetcher => {
            warn!(?service, task="FETCH", subtask="Missing Envelope Fetcher", event="SHUTDOWN", err=?a);
            envelope_processor.abort();
            latest_tip_fetcher.abort();
            a??
        }
    };
    // if any of the above selected, shut down this peer.
    envelope_processor.abort();
    latest_tip_fetcher.abort();
    missing_envelope_fetcher.abort();

    INFER_UNIT
}

/// enevelope processor verifies an envelope and then forwards any unknown tips
/// to the missing_envelope_fetcher.
pub(crate) fn envelope_processor(
    g: Arc<Globals>,
    service: &ServiceUrl,
    conn: MsgDB,
    mut next_envelope: tokio::sync::mpsc::UnboundedReceiver<(Vec<Envelope>, NotifyOnDrop)>,
    request_tips: UnboundedSender<Vec<CanonicalEnvelopeHash>>,
    allow_unsolicited_tips: bool,
) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    let service = service.clone();
    tokio::spawn(async move {
        while let Some((resp, cancel_inflight)) = next_envelope.recv().await {
            // Prefer to process envelopes
            handle_envelope(
                g.clone(),
                &service,
                resp,
                &conn,
                &request_tips,
                allow_unsolicited_tips,
                cancel_inflight,
            )
            .await?;
            if g.shutdown.should_quit() {
                break;
            }
        }
        INFER_UNIT
    })
}
async fn handle_envelope(
    g: Arc<Globals>,
    service: &ServiceUrl,
    resp: Vec<Envelope>,
    conn: &MsgDB,
    request_tips: &UnboundedSender<Vec<CanonicalEnvelopeHash>>,
    allow_unsolicited_tips: bool,
    _cancel_inflight: NotifyOnDrop,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut all_tips = Vec::new();
    for envelope in resp {
        if g.shutdown.should_quit() {
            break;
        }
        tracing::debug!(height = envelope.header().height(),
                        hash = ?envelope.canonicalized_hash_ref(),
                        genesis = ?envelope.get_genesis_hash(),
                        ?service,
                        "Processing this envelope");
        tracing::trace!(?envelope, ?service, "Processing this envelope");
        match envelope.self_authenticate(&g.secp) {
            Ok(authentic) => {
                tracing::debug!(?service, "Authentic Tip: {:?}", authentic);
                if authentic.inner_ref().header().ancestors().is_none()
                    && authentic.inner_ref().header().height() == 0
                {
                    let new_name = format!("user-{}", now());
                    let mut handle = conn.get_handle_all().await;
                    let res = spawn_blocking(move || {
                        handle.insert_user_by_genesis_envelope(new_name, authentic)
                    })
                    .await
                    .expect("DB Panic")?;
                    match res {
                        Ok(key) => {
                            trace!(key, ?service, "Created New Genesis From Peer");
                        }
                        Err((SqliteFail::SqliteConstraintUnique, _msg)) => {
                            trace!(?service, "Already Have this Chain");
                        }
                        Err(e) => {
                            warn!(err=?e, "Other SQL Error");
                            Err(format!("{:?}", e))?;
                        }
                    }
                } else {
                    let authentic_copy = authentic.clone();
                    let mut handle = conn.get_handle_all().await;
                    let res = spawn_blocking(move || {
                        handle.try_insert_authenticated_envelope(authentic_copy, false)
                    })
                    .await
                    .expect("DB Panic")?;
                    match res {
                        Ok(()) => {}
                        // This means that a conststraint, most likely that the
                        // genesis header must be known, was not allowed
                        Err((SqliteFail::SqliteConstraintCheck, _msg)) => {
                            // try fetching the missing tip
                            if allow_unsolicited_tips {
                                all_tips.push(envelope.get_genesis_hash());
                            }
                        }
                        Err((SqliteFail::SqliteConstraintUnique, _msg)) => {
                            trace!("Already Have this Envelope, Passing");
                        }
                        // This means that the constraint that the user ID was known
                        // was hit, so we need to attempt inserting as a genesis
                        // envelope
                        Err((SqliteFail::SqliteConstraintNotNull, msg)) => {
                            if allow_unsolicited_tips {
                                debug!(
                                    hash = ?authentic.inner_ref().canonicalized_hash_ref(),
                                    ?msg,
                                    "unsolicited tip received",
                                );
                                trace!(envelope=?authentic);
                                all_tips.push(envelope.get_genesis_hash());
                            }
                        }
                    }
                }
                // safe to reuse since it is authentic still..
                all_tips.extend(envelope.header().tips().iter().map(|(_, _, v)| *v));
                all_tips.extend(envelope.header().ancestors().iter().map(|a| a.prev_msg()));
            }
            Err(_) => {
                // TODO: Ban peer?
                tracing::warn!(hash=?envelope.canonicalized_hash_ref(), "Message Validation Failed");
                tracing::trace!(?envelope, "Message Validation Failed");
            }
        }
    }
    all_tips.sort_unstable();
    all_tips.dedup();
    let unknown_dep_tips = {
        let handle = conn.get_handle_read().await;
        // ideally we'd capture just handle and keep a ref to all_tips, but IDK
        // how to do that.
        let it = all_tips.clone();
        spawn_blocking(move || handle.message_not_exists_it(it.iter())).await??
    };
    trace!(?all_tips, ?unknown_dep_tips);
    if !unknown_dep_tips.is_empty() {
        request_tips.send(unknown_dep_tips)?;
    }
    Ok(())
}

/// latest_tip_fetcher periodically (randomly) pings a hidden service for it's
/// latest tips
pub(crate) fn latest_tip_fetcher(
    g: Arc<Globals>,
    client: AttestationClient,
    service: &ServiceUrl,
    envelopes_to_process: tokio::sync::mpsc::UnboundedSender<(Vec<Envelope>, NotifyOnDrop)>,
) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    let service = service.clone();
    tokio::spawn(async move {
        while !g.shutdown.should_quit() {
            let sp = tracing::debug_span!(
                "Fetching Latest Tips",
                ?service,
                task = "FETCH",
                subtask = "latest_tip_fetcher",
            );
            let _ = sp.enter();
            let resp: Vec<Envelope> = client
                .get_latest_tips(&service)
                .await
                .ok_or("Latest Tips Not Received")?;
            envelopes_to_process.send((resp, NotifyOnDrop::empty()))?;
            g.config.peer_service.timer_override.tip_fetch_delay().await;
        }
        INFER_UNIT
    })
}

/// missing_envelope_fetcher ingests a Vec<Hash> and queries a service for the envelope
/// of those hashes, then sends those envelopers for processing.
pub(crate) fn missing_envelope_fetcher(
    g: Arc<Globals>,
    client: AttestationClient,
    service: &ServiceUrl,
    envelopes_to_process: tokio::sync::mpsc::UnboundedSender<(Vec<Envelope>, NotifyOnDrop)>,
    mut tips_to_resolve: tokio::sync::mpsc::UnboundedReceiver<Vec<CanonicalEnvelopeHash>>,
) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    let service = service.clone();
    tokio::spawn(async move {
        while !g.shutdown.should_quit() {
            info!(?service, "waiting for tips to fetch");
            if let Some(tips) = tips_to_resolve.recv().await {
                info!(?service, n = tips.len(), "got tips to fetch");
                let (resp, remove_inflight) = client
                    .get_tips(Tips { tips }, &service, true)
                    .await
                    .ok_or("Tips Not Fetched")?;
                info!(?service, n = resp.len(), "got tips in response");
                envelopes_to_process.send((resp, remove_inflight))?;
            } else {
                info!("Terminating Tip Resolver");
                break;
            }
        }
        INFER_UNIT
    })
}
