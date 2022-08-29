use crate::{
    attestations::client::AttestationClient,
    control::{
        client::ControlClient,
        query::{Outcome, PushMsg, Subscribe},
    },
    init_main, BitcoinConfig, Config, ControlConfig,
};
use attest_messages::{CanonicalEnvelopeHash, Envelope};
use bitcoincore_rpc_async::Auth;
use futures::{future::join_all, stream::FuturesUnordered, Future, StreamExt};
use reqwest::Client;
use ruma_serde::CanonicalJsonValue;
use sapio_bitcoin::XOnlyPublicKey;
use serde_json::Value;
use std::{
    collections::BTreeSet,
    env::temp_dir,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use test_log::test;
use tokio::spawn;
use tracing::{debug, info};
const HOME: &'static str = "127.0.0.1";

// Connect to a specific local server for testing, or assume there is an
// open-to-world server available locally
fn get_btc_config() -> BitcoinConfig {
    match std::env::var("TEST_BTC_CONF") {
        Ok(s) => serde_json::from_str(&s).unwrap(),
        Err(_) => BitcoinConfig {
            url: "http://127.0.0.1".into(),
            auth: Auth::None,
        },
    }
}
fn get_test_id() -> Option<u16> {
    let test_one = std::env::var("ATTEST_TEST_ONE").is_ok();
    let test_two = std::env::var("ATTEST_TEST_TWO").is_ok();
    if !test_one && !test_two {
        tracing::debug!("Skipping Test, not enabled");
        return None;
    } else {
        tracing::debug!("One XOR Two? {}", test_one ^ test_two);
        assert!(test_one ^ test_two);
    }
    Some(if test_one { 0 } else { 1 })
}
async fn test_context<T, F>(nodes: u8, code: F) -> ()
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
    F: Fn(Vec<(u16, u16)>) -> T,
{
    let mut unord = FuturesUnordered::new();
    let mut quits = vec![];
    let mut ports = vec![];
    for test_id in 0..nodes {
        let btc_config = get_btc_config();
        let quit = Arc::new(AtomicBool::new(false));
        quits.push(quit.clone());
        let mut dir = temp_dir();
        let mut rng = sapio_bitcoin::secp256k1::rand::thread_rng();
        use sapio_bitcoin::secp256k1::rand::Rng;
        let bytes: [u8; 16] = rng.gen();
        use sapio_bitcoin::hashes::hex::ToHex;
        dir.push(format!("test-rust-{}", bytes.to_hex()));
        tracing::debug!("Using tmpdir: {}", dir.display());
        let dir = attest_util::ensure_dir(dir).await.unwrap();
        let timer_override = crate::PeerServicesTimers::scaled_default(0.1);
        let config = Config {
            bitcoin: btc_config.clone(),
            subname: format!("subname-{}", test_id),
            attestation_port: 12556 + test_id as u16,
            tor: None,
            control: ControlConfig {
                port: 14556 + test_id as u16,
            },
            prefix: Some(dir),
            peer_service: crate::PeerServiceConfig { timer_override },
        };
        ports.push((config.attestation_port, config.control.port));
        let task_one = spawn(async move { init_main(Arc::new(config), quit).await });
        unord.push(task_one);
    }

    let fail = tokio::select! {
        _ = code(ports) => {
            tracing::debug!("Main Task Completed");
            None
        }
        r = unord.next() => {
            tracing::debug!("Some Task Completed");
            r
        }
    };
    for quit in &quits {
        quit.store(true, Ordering::Relaxed);
    }
    // Wait for tasks to finish
    for _ in unord.next().await {}
    if fail.is_some() {
        fail.unwrap().unwrap().unwrap()
    }
}

#[test(tokio::test(flavor = "multi_thread", worker_threads = 5))]
async fn connect_and_test_nodes() {
    const NODES: u8 = 5;
    test_context(NODES, |ports| async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        // TODO: Guarantee all clients are started?
        let base = Client::new();
        let client = AttestationClient::new(base.clone());
        let control_client = ControlClient(base.clone());
        // Initial fetch should show no tips posessed
        {
            let it = ports.iter().map(|(port, _ctrl)| {
                let client = client.clone();
                async move { client.get_latest_tips(&HOME.into(), *port).await }
            });
            let resp = join_all(it).await;
            let empty = (0..NODES).map(|_| Some(vec![])).collect::<Vec<_>>();
            assert_eq!(resp.into_iter().map(|r| r.ok()).collect::<Vec<_>>(), empty);
        }
        // Create a genesis envelope for each node
        let genesis_envelopes = {
            let it = ports.iter().map(|(_port, ctrl)| {
                let control_client = control_client.clone();
                async move {
                    control_client
                        .make_genesis(&format!("ch-{}", ctrl), &HOME.into(), *ctrl)
                        .await
                }
            });
            let resp = join_all(it).await;
            debug!("Created {:?}", resp);
            let genesis_resp = resp
                .into_iter()
                .collect::<Result<Vec<Envelope>, _>>()
                .unwrap();
            genesis_resp
        };
        // Check that each node knows about it's own genesis envelope
        {
            let it = ports.iter().map(|(port, _ctrl)| {
                let client = client.clone();
                async move { client.get_latest_tips(&HOME.into(), *port).await }
            });
            let resp = join_all(it).await;
            debug!("Got {:?}", resp);
            assert_eq!(
                resp.into_iter()
                    .flat_map(|r| r.ok().unwrap())
                    .collect::<Vec<_>>(),
                genesis_envelopes
            );
        }

        // Add a message to each client like test-1-for-12345

        // Check that the messages are available as tips
        let check_synched = |n: u64, require_full: bool| {
            let ports = ports.clone();
            let client = client.clone();
            check_synched(n, require_full, ports, client)
        };

        make_nth(
            1,
            ports.clone(),
            control_client.clone(),
            genesis_envelopes.clone(),
        )
        .await;
        check_synched(1, false).await;
        // Connect each peer to every other peer
        {
            let futs = move |to, cli: ControlClient, ctrl: u16| async move {
                cli.add_service(
                    &Subscribe {
                        url: HOME.into(),
                        port: to,
                    },
                    &HOME.into(),
                    ctrl,
                )
                .await
            };
            let it = ports.iter().map(|(port, ctrl)| {
                let control_client = control_client.clone();
                let ports = ports.clone();
                async move {
                    let subbed: Vec<_> = join_all(
                        ports
                            .iter()
                            // don't connect to self
                            .filter(|(p, _)| p != port)
                            .map(|(port, _ctl)| futs(*port, control_client.clone(), *ctrl)),
                    )
                    .await
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                    subbed
                }
            });
            let resp = join_all(it).await;
            // No Failures
            assert!(resp.iter().flatten().all(|o| o.success));
            // handshaking lemma n*n-1 would be "cleaner", but this checks
            // more strictly each one had the right number of responses
            assert!(resp.iter().all(|v| v.len() == ports.len() - 1));
            debug!("All Connected");
            check_synched(1, true).await;
            info!("All Synchronized")
        }

        // TODO: signal that notifies after re-peering successful?
        // Get tips for all clients (doesn't depend on past bit processing yet)
        let mut old_tips: BTreeSet<_> = get_all_tips(ports.clone(), client.clone()).await;
        debug!(current_tips = ?old_tips, "Tips before adding a new message");
        // Create a new message on all chain tips
        make_nth(
            2,
            ports.clone(),
            control_client.clone(),
            genesis_envelopes.clone(),
        )
        .await;
        check_synched(2, true).await;

        // wait twice the time for our attach_tips to get called (TODO: have a non-race condition?)
        tokio::time::sleep(Duration::from_millis(2000)).await;

        test_envelope_inner_tips(ports.clone(), client.clone(), old_tips).await;

        let mut old_tips: BTreeSet<_> = get_all_tips(ports.clone(), client.clone()).await;
        debug!(current_tips = ?old_tips, "Tips before adding a new message");
        for x in 3..=10 {
            make_nth(
                x,
                ports.clone(),
                control_client.clone(),
                genesis_envelopes.clone(),
            )
            .await;
        }
        check_synched(10, true).await;
        tokio::time::sleep(Duration::from_millis(2000)).await;
        test_envelope_inner_tips(ports.clone(), client.clone(), old_tips).await;

        ()
    })
    .await
}

async fn get_all_tips(
    ports: Vec<(u16, u16)>,
    client: AttestationClient,
) -> BTreeSet<CanonicalEnvelopeHash> {
    let it = ports.iter().map(|(port, _ctrl)| {
        let client = client.clone();
        async move { client.get_latest_tips(&HOME.into(), *port).await }
    });
    let resp = join_all(it).await;
    resp.iter()
        .flatten()
        .flatten()
        .map(|c| c.canonicalized_hash_ref())
        .collect()
}
async fn test_envelope_inner_tips(
    ports: Vec<(u16, u16)>,
    client: AttestationClient,
    mut old_tips: BTreeSet<CanonicalEnvelopeHash>,
) {
    let mut new_tips = get_all_tips(ports.clone(), client.clone()).await;
    debug!(all_tips = ?new_tips, "Adding Tips");
    old_tips.append(&mut new_tips);

    debug!(all_tips = ?old_tips, "Tips to Check For");
    // Attempt to check that the latest tips of all clients Envelopes are in sync
    let it = ports.iter().map(|(port, _ctrl)| {
        let client = client.clone();
        async move { client.get_latest_tips(&HOME.into(), *port).await }
    });
    let resp = join_all(it).await;

    for r in &resp {
        for e in r.as_ref().unwrap() {
            debug!(envelope=?e, "Checking Tips On");
            let s = e
                .header()
                .tips()
                .iter()
                .map(|tip| tip.2)
                .collect::<BTreeSet<_>>();
            assert_eq!(s.len(), ports.len() - 1);
            let diff = s.difference(&old_tips);
            let diff: Vec<_> = diff.cloned().collect();
            assert_eq!(diff, vec![]);
            assert!(!s.contains(&e.header().ancestors().unwrap().prev_msg()));
        }
    }
}

fn nth_msg_per_port(port: u16, n: u64) -> CanonicalJsonValue {
    format!("test-{}-for-{}", n, port).into()
}
async fn make_nth(
    n: u64,
    ports: Vec<(u16, u16)>,
    control_client: ControlClient,
    genesis_envelopes: Vec<Envelope>,
) {
    let keys = genesis_envelopes
        .iter()
        .map(|g| g.header().key())
        .collect::<Vec<_>>();
    info!(n, "Making messages");
    let make_message = |((port, ctrl), key): ((u16, u16), XOnlyPublicKey)| {
        let control_client = control_client.clone();
        async move {
            control_client
                .push_message_dangerous(
                    &PushMsg {
                        key,
                        msg: nth_msg_per_port(port, n),
                    },
                    &HOME.into(),
                    ctrl,
                )
                .await
        }
    };
    let it = ports
        .iter()
        .cloned()
        .zip(keys.into_iter())
        .map(make_message);
    let resp = join_all(it).await;
    info!(n, "Made Messages: {:?}", resp);
    let pushmsg_resp = resp
        .into_iter()
        .collect::<Result<Vec<Outcome>, _>>()
        .unwrap();
    assert!(pushmsg_resp.iter().all(|v| v.success));
}

async fn check_synched(
    n: u64,
    require_full: bool,
    ports: Vec<(u16, u16)>,
    client: AttestationClient,
) {
    let mut expected = ports
        .iter()
        .map(|(port, _)| nth_msg_per_port(*port, n))
        .collect::<Vec<_>>();
    expected.sort_by(|k1, k2| k1.as_str().cmp(&k2.as_str()));
    'resync: for attempt in 0u32.. {
        info!(
            ?expected,
            attempt, require_full, "checking for synchronization"
        );
        let it = ports.iter().map(|(port, _ctrl)| {
            let client = client.clone();
            async move { client.get_latest_tips(&HOME.into(), *port).await }
        });
        let resp = join_all(it).await;
        info!("Initial Check that all nodes know their own message");
        for (r, (port, _)) in resp.iter().zip(ports.iter()) {
            let tips = r.as_ref().ok().unwrap();
            let needle = nth_msg_per_port(*port, n);
            info!(?port, response = ?tips.iter().map(|t| t.msg()).collect::<Vec<_>>(), seeking = ?needle, "Node Got Response");

            assert!(tips
                .iter()
                .map(|t| t.msg())
                .find(|f| f.as_str() == needle.as_str())
                .is_some())
        }
        if require_full {
            for r in resp {
                let tips = r.ok().unwrap();
                let mut msgs = tips
                    .into_iter()
                    .map(|m| m.msg().clone())
                    .collect::<Vec<_>>();
                msgs.sort_by(|k1, k2| k1.as_str().cmp(&k2.as_str()));
                if expected != msgs {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue 'resync;
                }
            }
        }
        break 'resync;
    }

    info!(n, "Synchronization success");
}
