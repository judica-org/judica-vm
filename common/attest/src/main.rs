use attest_database::connection::MsgDB;
use attest_database::setup_db;
use attest_util::INFER_UNIT;
use bitcoin_header_checkpoints::BitcoinCheckPointCache;
use bitcoincore_rpc_async as rpc;
use rpc::Client;
use sapio_bitcoin::secp256k1::rand::Rng;
use sapio_bitcoin::secp256k1::{rand, Secp256k1, Verification};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::channel;
use tokio::task::JoinHandle;
mod attestations;
mod control;
mod peer_services;
mod tor;

const fn default_port() -> u16 {
    46789
}
/// The different authentication methods for the client.
#[derive(Serialize, Deserialize)]
#[serde(remote = "rpc::Auth")]
pub enum Auth {
    None,
    UserPass(String, String),
    CookieFile(PathBuf),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BitcoinConfig {
    pub url: String,
    #[serde(with = "Auth")]
    pub auth: rpc::Auth,
}

fn default_socks_port() -> u16 {
    19050
}
#[derive(Serialize, Deserialize, Clone)]
pub struct TorConfig {
    directory: PathBuf,
    #[serde(default = "default_socks_port")]
    socks_port: u16,
}

fn default_control_port() -> u16 {
    14322
}
#[derive(Serialize, Deserialize)]
pub struct ControlConfig {
    #[serde(default = "default_control_port")]
    port: u16,
}
#[derive(Serialize, Deserialize)]
pub struct Config {
    bitcoin: BitcoinConfig,
    pub subname: String,
    pub tor: Option<TorConfig>,
    #[serde(default = "default_port")]
    pub attestation_port: u16,
    pub control: ControlConfig,
    #[serde(default)]
    pub prefix: Option<PathBuf>,
}

fn get_config() -> Result<Arc<Config>, Box<dyn Error>> {
    let config = std::env::var("ATTEST_CONFIG_JSON").map(|s| serde_json::from_str(&s))??;
    Ok(Arc::new(config))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let quit = Arc::new(AtomicBool::new(false));
    let args: Vec<String> = std::env::args().into_iter().collect();
    let config = match get_config() {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!("Trying to read config from file {}", e);
            if args.len() != 2 {
                Err("Expected only 2 args, file name of config")?;
            }
            let config: Arc<Config> = Arc::new(serde_json::from_slice(
                &tokio::fs::read(&args[1]).await?[..],
            )?);
            config
        }
    };
    init_main(config, quit).await
}
async fn init_main(
    config: Arc<Config>,
    quit: Arc<AtomicBool>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing::debug!("Config Loaded");
    let bitcoin_client =
        Arc::new(Client::new(config.bitcoin.url.clone(), config.bitcoin.auth.clone()).await?);
    tracing::debug!("Bitcoin Client Loaded");
    let bitcoin_checkpoints =
        Arc::new(BitcoinCheckPointCache::new(bitcoin_client, None, quit.clone()).await);
    let mut checkpoint_service = bitcoin_checkpoints
        .run_cache_service()
        .ok_or("Checkpoint service already started")?;
    tracing::debug!("Checkpoint Service Started");
    tracing::debug!("Opening DB");
    let application = format!("attestations.{}", config.subname);
    let mdb = setup_db(&application, config.prefix.clone())
        .await
        .map_err(|e| format!("{}", e))?;
    tracing::debug!("Database Connection Setup");
    let mut attestation_server = attestations::server::run(config.clone(), mdb.clone()).await;
    let mut tor_service = tor::start(config.clone()).await?;
    let (tx_peer_status, rx_peer_status) = channel(1);
    let mut fetching_client =
        peer_services::startup(config.clone(), mdb.clone(), quit.clone(), rx_peer_status);
    let mut control_server = control::server::run(
        config.clone(),
        mdb.clone(),
        tx_peer_status,
        bitcoin_checkpoints,
    )
    .await;

    tracing::debug!("Starting Subservices");
    let mut skip = None;
    let _to_skip = tokio::select!(
    a = &mut attestation_server => {
        tracing::debug!("Error From Attestation Server: {:?}", a);
        skip.replace("attest");
    },
    b = &mut tor_service => {
        tracing::debug!("Error From Tor Server: {:?}", b);
        skip.replace("tor");
    },
    c = &mut fetching_client => {
        tracing::debug!("Error From Fetching Server: {:?}", c);
        skip.replace("fetch");
    },
    d = &mut checkpoint_service => {
        tracing::debug!("Error From Checkpoint Server: {:?}", d);
        skip.replace("checkpoint");
    }
    e = &mut control_server => {
        tracing::debug!("Error From Control Server: {:?}", e);
        skip.replace("control");
    });
    tracing::debug!("Shutting Down Subservices");
    quit.store(true, Ordering::Relaxed);
    let svcs = [
        ("tor", tor_service),
        ("attest", attestation_server),
        ("fetch", fetching_client),
        ("checkpoint", checkpoint_service),
        ("control", control_server),
    ];
    for svc in &svcs {
        tracing::debug!("Abort Subservice: {}", svc.0);
        svc.1.abort();
    }
    futures::future::join_all(svcs.into_iter().filter_map(|x| {
        if Some(x.0) == skip {
            tracing::debug!("Skipping Wait for Terminated Subservice: {}", x.0);
            None
        } else {
            tracing::debug!("Waiting for Subservice: {}", x.0);
            Some(x.1)
        }
    }))
    .await;

    tracing::debug!("Exiting");
    INFER_UNIT
}

#[cfg(test)]
mod test {
    use std::{
        env::temp_dir,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::Duration,
    };

    use attest_messages::Envelope;
    use attest_util::INFER_UNIT;
    use bitcoincore_rpc_async::Auth;
    use futures::{future::join_all, stream::FuturesUnordered, Future, StreamExt};
    use reqwest::Client;
    use serde_json::Value;
    use test_log::test;
    use tokio::spawn;

    use crate::{
        attestations::client::AttestationClient,
        control::{
            client::ControlClient,
            query::{Outcome, PushMsg, Subscribe},
        },
        init_main, BitcoinConfig, Config, ControlConfig, TorConfig,
    };

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
            let config = Config {
                bitcoin: btc_config.clone(),
                subname: format!("subname-{}", test_id),
                attestation_port: 12556 + test_id as u16,
                tor: None,
                control: ControlConfig {
                    port: 14556 + test_id as u16,
                },
                prefix: Some(dir),
            };
            ports.push((config.attestation_port, config.control.port));
            let task_one = spawn(async move { init_main(Arc::new(config), quit).await });
            unord.push(task_one);
        }

        let mut fail = None;
        tokio::select! {
            _ = code(ports) => {
                tracing::debug!("Main Task Completed");
                return;
            }
            r = unord.next() => {
                tracing::debug!("Some Task Completed");
                fail = r;
            }
        };
        for quit in &quits {
            quit.store(false, Ordering::Relaxed);
        }
        // Wait for tasks to finish
        for _ in unord.next().await {}
        if fail.is_some() {
            fail.unwrap().unwrap().unwrap()
        }
    }

    #[test(tokio::test(flavor = "multi_thread", worker_threads = 5))]
    async fn sleep_for_five() {
        const NODES: u8 = 3;
        test_context(NODES, |ports| async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            // TODO: Guarantee all clients are started?
            let base = Client::new();
            let client = AttestationClient(base.clone());
            let control_client = ControlClient(base.clone());
            let it = ports.iter().map(|(port, ctrl)| {
                let client = client.clone();
                async move { client.get_latest_tips(&"127.0.0.1".into(), *port).await }
            });
            let resp = join_all(it).await;
            assert_eq!(
                resp.into_iter().map(|r| r.ok()).collect::<Vec<_>>(),
                vec![Some(vec![]), Some(vec![]), Some(vec![])]
            );
            let it = ports.iter().map(|(port, ctrl)| {
                let control_client = control_client.clone();
                async move {
                    control_client
                        .make_genesis(&format!("ch-{}", ctrl), &"127.0.0.1".into(), *ctrl)
                        .await
                }
            });
            let resp = join_all(it).await;
            println!("Created {:?}", resp);
            let genesis_resp = resp
                .into_iter()
                .collect::<Result<Vec<Envelope>, _>>()
                .unwrap();

            let it = ports.iter().map(|(port, ctrl)| {
                let client = client.clone();
                async move { client.get_latest_tips(&"127.0.0.1".into(), *port).await }
            });
            let resp = join_all(it).await;
            println!("Got {:?}", resp);
            assert_eq!(
                resp.into_iter()
                    .flat_map(|r| r.ok().unwrap())
                    .collect::<Vec<_>>(),
                genesis_resp
            );

            let it = ports
                .iter()
                .zip(genesis_resp.iter())
                .map(|((port, ctrl), genesis)| {
                    let control_client = control_client.clone();
                    let genesis = genesis.clone();
                    async move {
                        control_client
                            .push_message_dangerous(
                                &PushMsg {
                                    key: genesis.header.key,
                                    msg: "Test!".into(),
                                },
                                &"127.0.0.1".into(),
                                *ctrl,
                            )
                            .await
                    }
                });
            let resp = join_all(it).await;
            println!("Created {:?}", resp);
            let pushmsg_resp = resp
                .into_iter()
                .collect::<Result<Vec<Outcome>, _>>()
                .unwrap();

            let it = ports.iter().map(|(port, ctrl)| {
                let client = client.clone();
                async move { client.get_latest_tips(&"127.0.0.1".into(), *port).await }
            });
            let resp = join_all(it).await;
            println!("Got {:?}", resp);
            assert_eq!(
                resp.into_iter()
                    .flat_map(|r| r.ok().unwrap())
                    .map(|m| m.msg)
                    .collect::<Vec<_>>(),
                ports
                    .iter()
                    .map(|_| -> Value { "Test!".into() })
                    .collect::<Vec<_>>()
            );

            let it = ports.iter().map(|(port, ctrl)| {
                let control_client = control_client.clone();
                let ports = ports.clone();
                async move {
                    let mut futs = move |to, cli: ControlClient| async move {
                        cli.add_service(
                            &Subscribe {
                                url: "127.0.0.1".into(),
                                port: to,
                            },
                            &"127.0.0.1".into(),
                            *ctrl,
                        )
                        .await
                    };
                    let subbed: Vec<_> = join_all(
                        ports
                            .iter()
                            // don't connect to self
                            .filter(|(p, _)| p != port)
                            .map(|(port, ctl)| futs(*port, control_client.clone())),
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
            // handshaking lemma would be better, but this checks
            // more strictly each one had the right number of responses
            assert!(resp.iter().all(|v| v.len() == ports.len() - 1));
            println!("All Connected");
            ()
        })
        .await
    }
}
