// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    attestations::{
        client::{AttestationClient, ServiceUrl},
        server::protocol::GlobalSocketState,
    },
    configuration::{Config, PeerServicesTimers},
    configuration::{ControlConfig, PeerServiceConfig},
    control::{
        client::ControlClient,
        query::{NewGenesis, Outcome, PushMsg, Subscribe},
    },
    globals::Globals,
    init_main, AppShutdown,
};
use attest_messages::{CanonicalEnvelopeHash, Envelope};
use attest_util::bitcoin::BitcoinConfig;
use attest_util::CrossPlatformPermissions;
use bitcoincore_rpc_async::Auth;
use futures::{future::join_all, stream::FuturesUnordered, Future, StreamExt};

use ruma_serde::CanonicalJsonValue;
use sapio_bitcoin::{
    secp256k1::{All, Secp256k1},
    XOnlyPublicKey,
};
use std::{collections::BTreeSet, env::temp_dir, sync::Arc, time::Duration};
use test_log::test;
use tracing::{debug, info};
const HOME: &str = "127.0.0.1";

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
async fn test_context<T, F>(nodes: u8, secp: Arc<Secp256k1<All>>, code: F)
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
    F: Fn(Vec<(u16, u16)>, Arc<Globals>) -> T,
{
    let mut unord = FuturesUnordered::new();
    let mut quits = vec![];
    let mut ports = vec![];
    let mut client_globals = None;
    for test_id in 0..nodes + 1 {
        let (shutdown, config) = create_test_config(&mut quits, test_id).await;
        ports.push((config.attestation_port, config.control.port));
        let secp = secp.clone();
        let msg_db = config.setup_db().await.unwrap();
        let globals = Arc::new(Globals {
            config: Arc::new(config),
            shutdown,
            secp,
            client: Default::default(),
            msg_db,
            socket_state: GlobalSocketState::default(),
        });
        if test_id == nodes {
            client_globals = Some(globals.clone());
            ports.pop();
        }
        unord.push(init_main(globals));
    }

    let fail = tokio::select! {
        _ = code(ports, client_globals.unwrap()) => {
            tracing::debug!("Main Task Completed");
            None
        }
        r = unord.next() => {
            tracing::debug!("Some Task Completed");
            r
        }
    };
    for quit in &quits {
        quit.begin_shutdown();
    }
    // Wait for tasks to finish
    while (unord.next().await).is_some() {}
    if fail.is_some() {
        fail.unwrap().unwrap()
    }
}

async fn create_test_config(quits: &mut Vec<AppShutdown>, test_id: u8) -> (AppShutdown, Config) {
    let btc_config = get_btc_config();
    let shutdown = AppShutdown::new();
    quits.push(shutdown.clone());
    let mut dir = temp_dir();
    let mut rng = sapio_bitcoin::secp256k1::rand::thread_rng();
    use sapio_bitcoin::secp256k1::rand::Rng;
    let bytes: [u8; 16] = rng.gen();
    use sapio_bitcoin::hashes::hex::ToHex;
    dir.push(format!("test-rust-{}", bytes.to_hex()));
    tracing::debug!("Using tmpdir: {}", dir.display());
    let dir = attest_util::ensure_dir(dir, CrossPlatformPermissions::whatever())
        .await
        .unwrap();
    let timer_override = PeerServicesTimers::scaled_default(0.001);
    let config = Config {
        bitcoin: btc_config.clone(),
        subname: format!("subname-{}", test_id),
        attestation_port: 12556 + test_id as u16,
        tor: None,
        control: ControlConfig {
            port: 14556 + test_id as u16,
        },
        prefix: Some(dir),
        peer_service: PeerServiceConfig { timer_override },
        test_db: true,
    };
    (shutdown, config)
}

#[test(tokio::test(flavor = "multi_thread", worker_threads = 5))]
async fn connect_and_test_nodes() {
    const NODES: u8 = 5;
    let secp = Arc::new(Secp256k1::new());
    test_context(NODES, secp.clone(), |ports, test_node| async move {
        tokio::time::sleep(Duration::from_millis(10)).await;

        // TODO: Guarantee all clients are started?
        let client = test_node.get_client().await.unwrap();
        let control_client = ControlClient(client.client().clone());
        // Initial fetch should show no tips posessed
        loop {
            let it = ports.iter().map(|(port, _ctrl)| {
                let client = client.clone();
                async move {
                    client
                        .get_latest_tips(&ServiceUrl(HOME.to_owned().into(), *port))
                        .await
                }
            });
            let resp = join_all(it).await;
            let empty = (0..NODES).map(|_| Some(vec![])).collect::<Vec<_>>();
            if resp.iter().any(Option::is_none) {
                // Wait until all services are online
                continue;
            }
            assert_eq!(resp.into_iter().collect::<Vec<_>>(), empty);
            break;
        }

        info!(checkpoint = "Initial fetch showed no tips posessed");

        // Create a genesis envelope for each node
        let genesis_envelopes = {
            let it = ports.iter().map(|(_port, ctrl)| {
                let control_client = control_client.clone();
                async move {
                    control_client
                        .make_genesis(
                            &NewGenesis {
                                nickname: format!("ch-{}", ctrl),
                                msg: CanonicalJsonValue::Null,
                            },
                            &HOME.into(),
                            *ctrl,
                        )
                        .await
                }
            });
            let resp = join_all(it).await;
            debug!("Created {:?}", resp);

            resp.into_iter()
                .collect::<Result<Vec<Envelope>, _>>()
                .unwrap()
        };

        info!(checkpoint = "Created Genesis for each Node");
        // Check that each node knows about it's own genesis envelope
        loop {
            let it = ports.iter().map(|(port, _ctrl)| {
                let client = client.clone();
                async move {
                    client
                        .get_latest_tips(&ServiceUrl(HOME.to_owned().into(), *port))
                        .await
                }
            });
            let resp = join_all(it).await;
            debug!("Got {:?}", resp);
            if resp
                .into_iter()
                .flat_map(|r| r.unwrap())
                .collect::<Vec<_>>()
                == genesis_envelopes
            {
                break;
            }
        }
        info!(checkpoint = "Each Node Has Own Genesis");

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

        info!(checkpoint = "New Tips Appear Locally");

        // Connect each peer to every other peer
        {
            let futs = move |to, cli: ControlClient, ctrl: u16| async move {
                cli.add_service(
                    &Subscribe {
                        url: HOME.into(),
                        port: to,
                        fetch_from: Some(true),
                        push_to: Some(true),
                        allow_unsolicited_tips: Some(true),
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
            info!(checkpoint = "Peered each node to each other, unsynchronized");
            check_synched(1, true).await;
            info!("All Synchronized");
            info!(checkpoint = "Peered each node to each other and synchronized");
        }

        // TODO: signal that notifies after re-peering successful?
        // Get tips for all clients (doesn't depend on past bit processing yet)
        let old_tips: BTreeSet<_> = get_all_tips(ports.clone(), client.clone()).await;
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
        tokio::time::sleep(Duration::from_millis(20)).await;

        test_envelope_inner_tips(ports.clone(), client.clone(), old_tips).await;
        info!(checkpoint = "New Tips Synchronize after peering");

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
        let old_tips: BTreeSet<_> = get_all_tips(ports.clone(), client.clone()).await;
        debug!(current_tips = ?old_tips, "Tips before adding a new message");
        make_nth(
            11,
            ports.clone(),
            control_client.clone(),
            genesis_envelopes.clone(),
        )
        .await;
        check_synched(11, true).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
        test_envelope_inner_tips(ports.clone(), client.clone(), old_tips).await;
    })
    .await
}

async fn get_all_tips(
    ports: Vec<(u16, u16)>,
    client: AttestationClient,
) -> BTreeSet<CanonicalEnvelopeHash> {
    let it = ports.iter().map(|(port, _ctrl)| {
        let client = client.clone();
        async move {
            client
                .get_latest_tips(&ServiceUrl(HOME.to_owned().into(), *port))
                .await
        }
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
        async move {
            client
                .get_latest_tips(&ServiceUrl(HOME.to_owned().into(), *port))
                .await
        }
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
    CanonicalJsonValue::String(format!("test-{}-for-{}", n, port))
}
async fn make_nth(
    n: u64,
    ports: Vec<(u16, u16)>,
    control_client: ControlClient,
    genesis_envelopes: Vec<Envelope>,
) {
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
        .zip(genesis_envelopes.iter().map(|g| g.header().key()))
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
            async move {
                client
                    .get_latest_tips(&ServiceUrl(HOME.to_owned().into(), *port))
                    .await
            }
        });
        let resp = join_all(it).await;
        info!("Initial Check that all nodes know their own message");
        for (r, (port, _)) in resp.iter().zip(ports.iter()) {
            let tips = r.as_ref().unwrap();
            let needle = nth_msg_per_port(*port, n);
            info!(?port, response = ?tips.iter().map(|t| t.msg()).collect::<Vec<_>>(), seeking = ?needle, "Node Got Response");

            assert!(tips
                .iter()
                .map(|t| t.msg())
                .any(|f| f.as_str() == needle.as_str()))
        }
        if require_full {
            for r in resp {
                let tips = r.unwrap();
                let mut msgs = tips
                    .into_iter()
                    .map(|m| m.msg().clone())
                    .collect::<Vec<_>>();
                msgs.sort_by(|k1, k2| k1.as_str().cmp(&k2.as_str()));
                if expected != msgs {
                    info!(?expected, got=?msgs);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue 'resync;
                }
            }
        }
        break 'resync;
    }

    info!(n, "Synchronization success");
}
