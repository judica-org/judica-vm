use super::*;

pub async fn client_fetching(db: MsgDB) -> Result<(), Box<dyn std::error::Error>> {
    let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:19050")?;
    let client = reqwest::Client::builder().proxy(proxy).build()?;

    let secp = Arc::new(Secp256k1::new());
    tokio::spawn(async move {
        let mut task_set: HashMap<String, JoinHandle<Result<(), _>>> = HashMap::new();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            let mut create_services: HashSet<_> = db
                .get_handle()
                .await
                .get_all_hidden_services()?
                .into_iter()
                .collect();
            task_set.retain(
                |service_id, service| match create_services.take(service_id) {
                    Some(_) => true,
                    None => {
                        service.abort();
                        false
                    }
                },
            );
            for url in create_services.into_iter() {
                let client = client.clone();
                task_set.insert(
                    url.clone(),
                    tokio::spawn(poll_service_for_tips(secp.clone(), client, url, db.clone())),
                );
            }
        }
        Ok::<(), Box<dyn Error + Send + Sync + 'static>>(())
    });
    Ok(())
}

const INFER_UNIT: Result<(), Box<dyn Error + Send + Sync + 'static>> = Ok(());
async fn poll_service_for_tips<C: Verification + 'static>(
    secp: Arc<Secp256k1<C>>,
    client: reqwest::Client,
    url: String,
    conn: MsgDB,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<sha256::Hash>>();
    let (tx_envelope, mut rx_envelope) = tokio::sync::mpsc::unbounded_channel::<Vec<Envelope>>();
    // tip_resolver ingests a Vec<Hash> and queries a service for the envelope
    // of those hashes, then sends those envelopers for processing.
    let tip_resolver = {
        let client = client.clone();
        let url = url.clone();
        let tx_envelope = tx_envelope.clone();
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
    };

    // tip_fetcher periodically (randomly)
    // pings a hidden service for it's latest tips
    let tip_fetcher = {
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
    };
    // enevelope processor verifies an envelope and then
    // forwards any unknown tips to the tip_resolver.
    let envelope_processor = {
        let secp = secp.clone();
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
    let (a, b, c) = tokio::join!(tip_fetcher, tip_resolver, envelope_processor);
    a??;
    b??;
    c??;
    Ok::<(), Box<dyn Error + Send + Sync + 'static>>(())
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
    let sent_time_ms = util::now().ok_or("Unknown Time")?;
    let mut msg = Envelope {
        header: Header {
            height: 0,
            prev_msg: sha256::Hash::hash(&[]),
            tips: Vec::new(),
            next_nonce: nonce.get_public(&secp),
            key: keypair.public_key().x_only_public_key().0,
            sent_time_ms,
            unsigned: Unsigned {
                signature: Default::default(),
            },
        },
        msg: InnerMessage::Ping(sent_time_ms),
    };
    msg.sign_with(&keypair, &secp, nonce)?;
    Ok((secp, keypair, nonce, msg))
}
