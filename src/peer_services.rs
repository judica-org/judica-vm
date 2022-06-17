use tokio::{sync::mpsc::UnboundedSender, time::MissedTickBehavior};

use crate::attestations::messages::CanonicalEnvelopeHash;

use super::*;

pub async fn client_fetching(db: MsgDB) -> Result<(), Box<dyn std::error::Error>> {
    let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:19050")?;
    let client = reqwest::Client::builder().proxy(proxy).build()?;

    let secp = Arc::new(Secp256k1::new());
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut task_set: HashMap<String, JoinHandle<Result<(), _>>> = HashMap::new();
        loop {
            interval.tick().await;
            let mut create_services: HashSet<_> = db
                .get_handle()
                .await
                .get_all_hidden_services()?
                .into_iter()
                .collect();
            // Drop anything that is finished, we will re-add it later if still in create_services
            task_set.retain(|_k, v| !v.is_finished());
            // If it is no longer in our services DB, drop it / disconnect
            task_set.retain(
                |service_id, service| match create_services.take(service_id) {
                    Some(_) => true,
                    None => {
                        service.abort();
                        false
                    }
                },
            );
            // Open connections to all services on the list and put into our task set.
            for url in create_services.into_iter() {
                let client = client.clone();
                task_set.insert(
                    url.clone(),
                    tokio::spawn(make_peer(secp.clone(), client, url, db.clone())),
                );
            }
        }
        Ok::<(), Box<dyn Error + Send + Sync + 'static>>(())
    });
    Ok(())
}

/// Helps with type inference
const INFER_UNIT: Result<(), Box<dyn Error + Send + Sync + 'static>> = Ok(());

async fn make_peer<C: Verification + 'static>(
    secp: Arc<Secp256k1<C>>,
    client: reqwest::Client,
    url: String,
    conn: MsgDB,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Vec<CanonicalEnvelopeHash>>();
    let (tx_envelope, rx_envelope) = tokio::sync::mpsc::unbounded_channel::<Vec<Envelope>>();

    let mut envelope_processor = envelope_processor(conn, secp, rx_envelope, tx);
    let mut tip_resolver = tip_resolver(client.clone(), url.clone(), tx_envelope.clone(), rx);
    let mut tip_fetcher = tip_fetcher(client, url, tx_envelope);
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
fn envelope_processor<C: Verification + 'static>(
    conn: MsgDB,
    secp: Arc<Secp256k1<C>>,
    mut rx_envelope: tokio::sync::mpsc::UnboundedReceiver<Vec<Envelope>>,
    tx: UnboundedSender<Vec<CanonicalEnvelopeHash>>,
) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    let envelope_processor = {
        tokio::spawn(async move {
            loop {
                if let Some(resp) = rx_envelope.recv().await {
                    let mut all_tips = Vec::new();
                    for envelope in resp {
                        tracing::debug!("Response: {:?}", envelope);
                        match envelope.self_authenticate(secp.as_ref()) {
                            Ok(authentic) => {
                                tracing::debug!("Authentic Tip: {:?}", authentic);
                                conn.get_handle()
                                    .await
                                    .try_insert_authenticated_envelope(authentic)?;
                                // safe to reuse since it is authentic still..
                                all_tips
                                    .extend(envelope.header.tips.iter().map(|(_, _, v)| v.clone()))
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
                    tx.send(unknown_dep_tips)?;
                }
            }
            INFER_UNIT
        })
    };
    envelope_processor
}

/// tip_fetcher periodically (randomly) pings a hidden service for it's
/// latest tips
fn tip_fetcher(
    client: reqwest::Client,
    url: String,
    tx_envelope: tokio::sync::mpsc::UnboundedSender<Vec<Envelope>>,
) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    let client = client.clone();
    let url = url.clone();
    tokio::spawn(async move {
        loop {
            tracing::debug!("Sending message...");
            let resp: Vec<Envelope> = client
                .get(format!("http://{}:{}/tips", url, PORT))
                .send()
                .await?
                .json()
                .await?;
            tx_envelope.send(resp)?;
            let d = Duration::from_secs(15)
                + Duration::from_millis(rand::thread_rng().gen_range(0, 1000));
            tokio::time::sleep(d).await;
        }
        INFER_UNIT
    })
}

/// tip_resolver ingests a Vec<Hash> and queries a service for the envelope
/// of those hashes, then sends those envelopers for processing.
fn tip_resolver(
    client: reqwest::Client,
    url: String,
    tx_envelope: tokio::sync::mpsc::UnboundedSender<Vec<Envelope>>,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<Vec<CanonicalEnvelopeHash>>,
) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    tokio::spawn(async move {
        loop {
            if let Some(tips) = rx.recv().await {
                let resp: Vec<Envelope> = client
                    .get(format!("http://{}:{}/tips", url, PORT))
                    .query(&Tips { tips })
                    .send()
                    .await?
                    .json()
                    .await?;
                tx_envelope.send(resp)?;
            }
        }
        INFER_UNIT
    })
}

fn generate_new_user() -> Result<
    (
        Secp256k1<sapio_bitcoin::secp256k1::All>,
        KeyPair,
        PrecomittedNonce,
        Envelope,
    ),
    Box<dyn Error>,
> {
    let secp = Secp256k1::new();
    let keypair: _ = KeyPair::new(&secp, &mut rand::thread_rng());
    let nonce = PrecomittedNonce::new(&secp);
    let sent_time_ms = util::now();
    let mut msg = Envelope {
        header: Header {
            height: 0,
            prev_msg: CanonicalEnvelopeHash::genesis(),
            tips: Vec::new(),
            next_nonce: nonce.get_public(&secp),
            key: keypair.public_key().x_only_public_key().0,
            sent_time_ms,
            unsigned: Unsigned {
                signature: Default::default(),
            },
            checkpoints: Default::default(),
        },
        msg: InnerMessage::Ping(sent_time_ms),
    };
    msg.sign_with(&keypair, &secp, nonce)?;
    Ok((secp, keypair, nonce, msg))
}
