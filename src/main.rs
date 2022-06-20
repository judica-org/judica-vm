use crate::attestations::messages::{Envelope, Header, Unsigned};
use crate::attestations::nonce::PrecomittedNonce;
use attestations::db::connection::MsgDB;
use attestations::messages::checkpoints::BitcoinCheckPointCache;
use attestations::server::Tips;
use rpc::Client;
use rusqlite::Connection;
use sapio_bitcoin::secp256k1::rand::Rng;
use sapio_bitcoin::secp256k1::{rand, Secp256k1, Verification};
use sapio_bitcoin::util::key::KeyPair;
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
mod util;

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
#[derive(Serialize, Deserialize)]
pub struct Config {
    bitcoin: BitcoinConfig,
    #[serde(default = "default_port")]
    pub attestation_port: u16,
    pub subname: String,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let quit = Arc::new(AtomicBool::new(false));
    let args: Vec<String> = std::env::args().into_iter().collect();
    if args.len() != 2 {
        Err("Expected only 2 args, file name of config")?;
    }
    let config: Arc<Config> = Arc::new(serde_json::from_slice(
        &tokio::fs::read(&args[1]).await?[..],
    )?);
    let bitcoin_client =
        Arc::new(Client::new(config.bitcoin.url.clone(), config.bitcoin.auth.clone()).await?);
    let bitcoin_checkpoints = BitcoinCheckPointCache::new(bitcoin_client, None, quit.clone())
        .await?
        .ok_or("Failed to create a cache")?;
    let mut checkpoint_service = bitcoin_checkpoints
        .run_cache_service()
        .await
        .ok_or("Checkpoint service already started")?;
    let application = format!("attestations.{}", config.subname);
    let dirs = directories::ProjectDirs::from("org", "judica", &application).unwrap();

    let data_dir: PathBuf = ensure_dir(dirs.data_dir().into()).await?;
    let mut chat_db_file = data_dir.clone();
    chat_db_file.push("chat.sqlite3");
    let mdb = MsgDB::new(Arc::new(tokio::sync::Mutex::new(
        Connection::open(chat_db_file).unwrap(),
    )));
    mdb.get_handle().await.setup_tables();

    let mut attestation_server =
        attestations::server::run(config.attestation_port, mdb.clone()).await;
    let mut tor_service = tor::start(data_dir.clone(), config.attestation_port);
    let mut fetching_client = peer_services::client_fetching(config.clone(), mdb.clone());

    tokio::select!(
        _ = &mut attestation_server => {},
        _ = &mut tor_service => {},
        _ = &mut fetching_client => {},
        _ = &mut checkpoint_service => {});
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

async fn ensure_dir(data_dir: PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    let dir = tokio::fs::create_dir_all(&data_dir).await;
    match dir.as_ref().map_err(std::io::Error::kind) {
        Err(std::io::ErrorKind::AlreadyExists) => (),
        _e => dir?,
    };
    Ok(data_dir)
}
