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
#[derive(Serialize, Deserialize)]
pub struct TorConfig {
    directory: PathBuf,
    #[serde(default = "default_port")]
    pub attestation_port: u16,
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
    pub tor: TorConfig,
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
    let mut control_server = control::run(
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

    use attest_util::INFER_UNIT;
    use bitcoincore_rpc_async::Auth;
    use futures::future::join_all;
    use reqwest::Client;
    use test_log::test;
    use tokio::spawn;

    use crate::{
        attestations::client::AttestationClient, init_main, BitcoinConfig, Config, ControlConfig,
        TorConfig,
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
    macro_rules! test_setup {
        {$name:ident, $code:tt} => {
    #[test(tokio::test(flavor = "multi_thread", worker_threads = 5))]
    async fn $name() {
        let test_id = if let Some(tid) = get_test_id() {
            tid
        } else {
            return
        };
        let btc_config = get_btc_config();
        let quit = Arc::new(AtomicBool::new(false));
        let mut dir = temp_dir();
        let mut rng = sapio_bitcoin::secp256k1::rand::thread_rng();
        use sapio_bitcoin::secp256k1::rand::Rng;
        let bytes : [u8; 16] = rng.gen();
        use sapio_bitcoin::hashes::hex::ToHex;
        dir.push(format!("test-rust-{}",bytes.to_hex()));
        tracing::debug!("Using tmpdir: {}", dir.display());
        let tor_dir = dir.join("tor");
        let config = Config {
            bitcoin: btc_config.clone(),
            subname: format!("subname-{}", test_id),
            tor: TorConfig {
                directory: tor_dir,
                attestation_port: 12556 + test_id,
                socks_port: 13556 + test_id,
            },
            control: ControlConfig { port: 14556 +test_id },
            prefix: Some(dir),
        };

        let main_task = {
            let quit = quit.clone();
            spawn(async move {
                $code
                quit.store(true, Ordering::Relaxed);
                ()
            })
        };
        let quit = quit.clone();
        let task_one = spawn(async  move{
            init_main(Arc::new(config), quit).await
        });
        tokio::select!{
            _ = main_task => {
                tracing::debug!("Main Task Completed");
                return;
            }
            r = task_one => {
                tracing::debug!("Task One Completed");
                r.unwrap().unwrap();
            }
        };
    }

        };
    }

    test_setup!(sleep_for_five, {
        let test_id = if let Some(tid) = get_test_id() {
            tid
        } else {
            return
        };
        tokio::time::sleep(Duration::from_secs(5)).await;
        let base = Client::new();
        let client = AttestationClient(base.clone());
        for _ in 0..10 {
            let resp = client
                .get_latest_tips(&"127.0.0.1".into(), 12556 + test_id)
                .await;
            tracing::debug!("Got:{:?}", resp);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}
