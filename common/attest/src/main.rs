use attest_database::connection::MsgDB;
use attest_database::setup_db;
use attest_messages::checkpoints::BitcoinCheckPointCache;
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

use bitcoincore_rpc_async as rpc;
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

#[derive(Serialize, Deserialize)]
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
}

fn get_config() -> Result<Arc<Config>, Box<dyn Error>> {
    let config = std::env::var("ATTEST_CONFIG_JSON").map(|s| serde_json::from_str(&s))??;
    Ok(Arc::new(config))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    tracing::debug!("Config Loaded");
    let bitcoin_client =
        Arc::new(Client::new(config.bitcoin.url.clone(), config.bitcoin.auth.clone()).await?);
    tracing::debug!("Bitcoin Client Loaded");
    let bitcoin_checkpoints = BitcoinCheckPointCache::new(bitcoin_client, None, quit.clone())
        .await?
        .ok_or("Failed to create a cache")?;
    let mut checkpoint_service = bitcoin_checkpoints
        .run_cache_service()
        .await
        .ok_or("Checkpoint service already started")?;
    tracing::debug!("Checkpoint Service Started");
    let application = format!("attestations.{}", config.subname);
    let mdb = setup_db(&application).await?;
    tracing::debug!("Database Connection Setup");
    let mut attestation_server = attestations::server::run(config.clone(), mdb.clone()).await;
    let mut tor_service = tor::start(config.clone());
    let (tx_peer_status, rx_peer_status) = channel(1);
    let mut fetching_client =
        peer_services::startup(config.clone(), mdb.clone(), quit.clone(), rx_peer_status);
    let mut control_server = control::run(config.clone(), mdb.clone(), tx_peer_status).await;

    tracing::debug!("Starting Subservices");
    let mut skip = "";
    let to_skip = tokio::select!(
    a = &mut attestation_server => {
        tracing::debug!("Error From Attestation Server: {:?}", a);
        skip = "attest";

    },
    b = &mut tor_service => {
        tracing::debug!("Error From Tor Server: {:?}", b);
        skip ="tor";
    },
    c = &mut fetching_client => {
        tracing::debug!("Error From Fetching Server: {:?}", c);
        skip = "fetch";
    },
    d = &mut checkpoint_service => {
        tracing::debug!("Error From Checkpoint Server: {:?}", d);
        skip = "checkpoint";
    }
    e = &mut control_server => {
        tracing::debug!("Error From Control Server: {:?}", e);
        skip = "control";
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
        if x.0 == skip {
            tracing::debug!("Skipping Wait for Terminated Subservice: {}", x.0);
            None
        } else {
            tracing::debug!("Waiting for Subservice: {}", x.0);
            Some(x.1)
        }
    }))
    .await;

    tracing::debug!("Exiting");
    Ok(())
}
