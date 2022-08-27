use crate::attestations::client::AttestationClient;
use crate::attestations::query::Tips;

use super::*;
use attest_database::db_handle::insert::SqliteFail;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::Envelope;
use attest_util::now;
use attest_util::INFER_UNIT;
use sapio_bitcoin::hashes::hex::ToHex;
use tokio;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Notify;
use tokio::time::MissedTickBehavior;
use tracing::info;

pub(crate) async fn fetch_from_peer<C: Verification + 'static>(
    config: Arc<Config>,
    secp: Arc<Secp256k1<C>>,
    client: AttestationClient,
    url: (String, u16),
    conn: MsgDB,
    allow_unsolicited_tips: bool,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Vec<CanonicalEnvelopeHash>>();
    let (tx_envelope, rx_envelope) = tokio::sync::mpsc::unbounded_channel::<Vec<Envelope>>();

    let mut envelope_processor = envelope_processor(
        config.clone(),
        conn,
        secp,
        rx_envelope,
        tx,
        allow_unsolicited_tips,
    );
    let mut tip_resolver = tip_resolver(
        config.clone(),
        client.clone(),
        url.clone(),
        tx_envelope.clone(),
        rx,
    );
    let mut tip_fetcher = tip_fetcher(config.clone(), client, url, tx_envelope);
    let _: () = tokio::select! {
        a = &mut envelope_processor => {a??}
        a = &mut tip_fetcher => {a??}
        a = &mut tip_resolver => {a??}
    };
    // if any of the above selected, shut down this peer.
    envelope_processor.abort();
    tip_fetcher.abort();
    tip_resolver.abort();

    INFER_UNIT
}

/// enevelope processor verifies an envelope and then forwards any unknown tips
/// to the tip_resolver.
pub(crate) fn envelope_processor<C: Verification + 'static>(
    config: Arc<Config>,
    conn: MsgDB,
    secp: Arc<Secp256k1<C>>,
    mut rx_envelope: tokio::sync::mpsc::UnboundedReceiver<Vec<Envelope>>,
    tx: UnboundedSender<Vec<CanonicalEnvelopeHash>>,
    allow_unsolicited_tips: bool,
) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    let envelope_processor = {
        tokio::spawn(async move {
            // We poll this is a biased order so we favour loading more data
            // before attaching tips
            let wake_if_no_work_left = Notify::new();
            // One initial permit, to let the attach_tips method enter first,
            // one time

            let mut interval = config.peer_service.timer_override.attach_tip_while_busy_interval();
            wake_if_no_work_left.notify_one();
            loop {
                tokio::select! {
                    biased;
                    // Try to tick once every 30 seconds with high priority if it doesn't happen naturally
                    _ = interval.tick() => {
                        conn.get_handle().await.attach_tips()?;
                    }
                    // Prefer to process envelopes
                    resp = rx_envelope.recv() => {
                        handle_envelope(resp, secp.as_ref(), &conn, &tx, &wake_if_no_work_left, allow_unsolicited_tips).await?;
                    }
                    _ = wake_if_no_work_left.notified() => {
                        conn.get_handle().await.attach_tips()?;
                        // Reset the tick since we just did the work.
                        interval.reset();
                    }
                }
            }
            // INFER_UNIT
        })
    };
    envelope_processor
}
async fn handle_envelope<C: Verification + 'static>(
    resp: Option<Vec<Envelope>>,
    secp: &Secp256k1<C>,
    conn: &MsgDB,
    tx: &UnboundedSender<Vec<CanonicalEnvelopeHash>>,
    wake_if_no_work_left: &Notify,
    allow_unsolicited_tips: bool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(resp) = resp {
        let mut all_tips = Vec::new();
        for envelope in resp {
            tracing::debug!("Response: {:?}", envelope);
            match envelope.self_authenticate(secp) {
                Ok(authentic) => {
                    tracing::debug!("Authentic Tip: {:?}", authentic);
                    let handle = conn.get_handle().await;
                    match handle.try_insert_authenticated_envelope(authentic.clone())? {
                        Ok(_) => {}
                        Err(SqliteFail::SqliteConstraintNotNull) => {
                            if allow_unsolicited_tips {
                                info!(
                                    "unsolicited tip received: {}",
                                    authentic
                                        .inner_ref()
                                        .canonicalized_hash_ref()
                                        .unwrap()
                                        .to_hex()
                                );
                                handle.insert_user_by_genesis_envelope(
                                    format!("user-{}", now()),
                                    authentic,
                                )??;
                            }
                        }
                        _ => {}
                    }
                    // safe to reuse since it is authentic still..
                    all_tips.extend(envelope.header.tips.iter().map(|(_, _, v)| v.clone()))
                }
                Err(_) => {
                    // TODO: Ban peer?
                    tracing::debug!("Invalid Tip: {:?}", envelope);
                }
            }
        }
        all_tips.sort_unstable();
        all_tips.dedup();
        let unknown_dep_tips = conn
            .get_handle()
            .await
            .message_not_exists_it(all_tips.iter())?;
        if !unknown_dep_tips.is_empty() {
            tx.send(unknown_dep_tips)?;
        } else {
            wake_if_no_work_left.notify_one();
        }
        Ok(())
    } else {
        return Ok(());
    }
}

/// tip_fetcher periodically (randomly) pings a hidden service for it's
/// latest tips
pub(crate) fn tip_fetcher(
    config: Arc<Config>,
    client: AttestationClient,
    (url, port): (String, u16),
    tx_envelope: tokio::sync::mpsc::UnboundedSender<Vec<Envelope>>,
) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    let client = client.clone();
    let url = url.clone();
    tokio::spawn(async move {
        loop {
            tracing::debug!("Sending message...");
            let resp: Vec<Envelope> = client.get_latest_tips(&url, port).await?;
            tx_envelope.send(resp)?;
            config.peer_service.timer_override.tip_fetch_delay().await;
        }
        // INFER_UNIT
    })
}

/// tip_resolver ingests a Vec<Hash> and queries a service for the envelope
/// of those hashes, then sends those envelopers for processing.
pub(crate) fn tip_resolver(
    config: Arc<Config>,
    client: AttestationClient,
    service: (String, u16),
    tx_envelope: tokio::sync::mpsc::UnboundedSender<Vec<Envelope>>,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<Vec<CanonicalEnvelopeHash>>,
) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    tokio::spawn(async move {
        loop {
            info!(?service, "waiting for tips to fetch");
            let (url, port) = &service;
            if let Some(tips) = rx.recv().await {
                info!(?service, "got {} tips to fetch", tips.len());
                let resp = client.get_tips(Tips { tips }, url, *port).await?;
                info!(?service, "got {} tips in response", resp.len());
                tx_envelope.send(resp)?;
            }
        }
        // INFER_UNIT
    })
}
