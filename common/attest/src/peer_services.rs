use tokio::{sync::mpsc::error::TryRecvError, time::MissedTickBehavior};

use attest_util::INFER_UNIT;

use super::*;

#[derive(Hash, Eq, Ord, PartialEq, PartialOrd, Copy, Clone)]
pub enum PeerType {
    Push,
    Fetch,
}
pub fn client_fetching(
    config: Arc<Config>,
    db: MsgDB,
) -> JoinHandle<Result<(), Box<dyn Error + Sync + Send + 'static>>> {
    let jh = tokio::spawn(async move {
        let proxy = reqwest::Proxy::all(format!("socks5h://127.0.0.1:{}", config.tor.socks_port))?;
        let client = reqwest::Client::builder().proxy(proxy).build()?;
        let secp = Arc::new(Secp256k1::new());
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        type Inboundness = bool;
        let mut task_set: HashMap<(String, u16, PeerType), JoinHandle<Result<(), _>>> =
            HashMap::new();
        loop {
            interval.tick().await;
            let mut create_services: HashSet<_> = db
                .get_handle()
                .await
                .get_all_hidden_services()?
                .into_iter()
                .flat_map(|p| {
                    let mut v = [None, None];
                    if p.fetch_from {
                        v[0] = Some((p.service_url.clone(), p.port, PeerType::Fetch))
                    }
                    if p.push_to {
                        v[1] = Some((p.service_url, p.port, PeerType::Push))
                    }
                    v
                })
                .flatten()
                .collect();
            // Drop anything that is finished, we will re-add it later if still in create_services
            task_set.retain(|_k, v| !v.is_finished());
            // If it is no longer in our services DB, drop it / disconnect, and also
            // remove it from our to create set (only if it is in the to retain set, mind you)
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
                match url.2 {
                    PeerType::Push => {
                        task_set.insert(
                            url.clone(),
                            tokio::spawn(push_peer::push_to_peer(
                                secp.clone(),
                                client,
                                (url.0, url.1),
                                db.clone(),
                            )),
                        );
                    }
                    PeerType::Fetch => {
                        task_set.insert(
                            url.clone(),
                            tokio::spawn(fetch_peer::fetch_from_peer(
                                secp.clone(),
                                client,
                                (url.0, url.1),
                                db.clone(),
                            )),
                        );
                    }
                }
            }
        }
        // INFER_UNIT
    });
    jh
}

mod push_peer {
    use super::*;
    pub async fn push_to_peer<C: Verification + 'static>(
        secp: Arc<Secp256k1<C>>,
        client: reqwest::Client,
        url: (String, u16),
        conn: MsgDB,
    ) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        Ok(())
    }
}

mod fetch_peer {
    use super::*;
    use attest_messages::CanonicalEnvelopeHash;
    use attest_messages::Envelope;
    use attest_util::INFER_UNIT;
    use tokio;
    use tokio::sync::mpsc::UnboundedSender;
    use tokio::sync::Notify;
    use tokio::time::MissedTickBehavior;

    pub(crate) async fn fetch_from_peer<C: Verification + 'static>(
        secp: Arc<Secp256k1<C>>,
        client: reqwest::Client,
        url: (String, u16),
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
    pub(crate) fn envelope_processor<C: Verification + 'static>(
        conn: MsgDB,
        secp: Arc<Secp256k1<C>>,
        mut rx_envelope: tokio::sync::mpsc::UnboundedReceiver<Vec<Envelope>>,
        tx: UnboundedSender<Vec<CanonicalEnvelopeHash>>,
    ) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
        let envelope_processor = {
            tokio::spawn(async move {
                // We poll this is a biased order so we favour loading more data
                // before attaching tips
                let wake_if_no_work_left = Notify::new();
                // One initial permit, to let the attach_tips method enter first,
                // one time

                let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
                interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
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
                            if let Some(resp) = resp {
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
                                            all_tips.extend(
                                                envelope.header.tips.iter().map(|(_, _, v)| v.clone()),
                                            )
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
                                wake_if_no_work_left.notify_one();

                            } else {
                                return Ok(());
                            }
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

    /// tip_fetcher periodically (randomly) pings a hidden service for it's
    /// latest tips
    pub(crate) fn tip_fetcher(
        client: reqwest::Client,
        (url, port): (String, u16),
        tx_envelope: tokio::sync::mpsc::UnboundedSender<Vec<Envelope>>,
    ) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
        let client = client.clone();
        let url = url.clone();
        tokio::spawn(async move {
            loop {
                tracing::debug!("Sending message...");
                let resp: Vec<Envelope> = client
                    .get(format!("http://{}:{}/tips", url, port))
                    .send()
                    .await?
                    .json()
                    .await?;
                tx_envelope.send(resp)?;
                let d = Duration::from_secs(15)
                    + Duration::from_millis(rand::thread_rng().gen_range(0, 1000));
                tokio::time::sleep(d).await;
            }
            // INFER_UNIT
        })
    }

    /// tip_resolver ingests a Vec<Hash> and queries a service for the envelope
    /// of those hashes, then sends those envelopers for processing.
    pub(crate) fn tip_resolver(
        client: reqwest::Client,
        (url, port): (String, u16),
        tx_envelope: tokio::sync::mpsc::UnboundedSender<Vec<Envelope>>,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<Vec<CanonicalEnvelopeHash>>,
    ) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
        tokio::spawn(async move {
            loop {
                if let Some(tips) = rx.recv().await {
                    let resp: Vec<Envelope> = client
                        .get(format!("http://{}:{}/tips", url, port))
                        .query(&Tips { tips })
                        .send()
                        .await?
                        .json()
                        .await?;
                    tx_envelope.send(resp)?;
                }
            }
            // INFER_UNIT
        })
    }
}
