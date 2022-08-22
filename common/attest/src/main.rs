use attest_database::connection::MsgDB;
use attest_database::setup_db;
use attest_messages::checkpoints::BitcoinCheckPointCache;
use attestations::server::Tips;
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
use tokio::task::JoinHandle;
mod attestations;
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
#[derive(Serialize, Deserialize)]
pub struct Config {
    bitcoin: BitcoinConfig,
    pub subname: String,
    pub tor: TorConfig,
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
    let mut fetching_client = peer_services::client_fetching(config.clone(), mdb.clone());

    tracing::debug!("Starting Subservices");
    tokio::select!(
    a = &mut attestation_server => {
        a?
    },
    b = &mut tor_service => {
        b?
    },
    c = &mut fetching_client => {
        c?
    },
    d = &mut checkpoint_service => {
        d?
    })
    .map_err(|e| format!("{}", e))?;
    quit.store(true, Ordering::Relaxed);
    attestation_server.abort();
    tor_service.abort();
    fetching_client.abort();
    checkpoint_service.abort();
    futures::future::join_all([
        tor_service,
        attestation_server,
        fetching_client,
        checkpoint_service,
    ])
    .await;
    Ok(())
}
