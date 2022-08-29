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
use tokio::time::{Interval, MissedTickBehavior};
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
pub struct PeerServicesTimers {
    pub reconnect_rate: Duration,
    pub scan_for_unsent_tips_rate: Duration,
    pub attach_tip_while_busy_rate: Duration,
    pub tip_fetch_rate: Duration,
    pub entropy_range: Duration,
}

impl PeerServicesTimers {
    fn scaled_default(scale: f64) -> Self {
        Self {
            reconnect_rate: Duration::from_millis((30000 as f64 * scale) as u64),
            scan_for_unsent_tips_rate: Duration::from_millis((10000 as f64 * scale) as u64),
            attach_tip_while_busy_rate: Duration::from_millis((30000 as f64 * scale) as u64),
            tip_fetch_rate: Duration::from_millis((15000 as f64 * scale) as u64),
            entropy_range: Duration::from_millis((1000 as f64 * scale) as u64),
        }
    }
}
impl Default for PeerServicesTimers {
    fn default() -> Self {
        Self::scaled_default(1.0)
    }
}
impl PeerServicesTimers {
    fn rand(&self) -> Duration {
        rand::thread_rng().gen_range(Duration::ZERO, self.entropy_range)
    }
    fn reconnect_interval(&self) -> Interval {
        let mut interval = tokio::time::interval(self.reconnect_rate);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        interval
    }
    async fn scan_for_unsent_tips_delay(&self) {
        let d = self.scan_for_unsent_tips_rate + self.rand();
        tokio::time::sleep(d).await
    }
    async fn tip_fetch_delay(&self) {
        let d = self.tip_fetch_rate + self.rand();
        tokio::time::sleep(d).await
    }
    // todo: add randomization
    fn attach_tip_while_busy_interval(&self) -> Interval {
        let mut interval = tokio::time::interval(self.attach_tip_while_busy_rate);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        interval
    }
}

#[derive(Serialize, Deserialize)]
pub struct PeerServiceConfig {
    #[serde(default)]
    pub timer_override: PeerServicesTimers,
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
    pub peer_service: PeerServiceConfig,
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
mod test;
