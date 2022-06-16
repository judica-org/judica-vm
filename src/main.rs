use crate::attestations::messages::{Envelope, Header, InnerMessage, Unsigned};
use crate::attestations::nonce::PrecomittedNonce;
use attestations::db::connection::MsgDB;
use attestations::server::Tips;
use ruma_signatures::Ed25519KeyPair;
use rusqlite::Connection;
use sapio_bitcoin::hashes::{sha256, Hash};
use sapio_bitcoin::secp256k1::rand::Rng;
use sapio_bitcoin::secp256k1::{rand, Secp256k1, Signing, Verification};
use sapio_bitcoin::util::key::KeyPair;
use sapio_bitcoin::{hashes::hex::ToHex, util::key};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
mod attestations;
mod peer_services;
mod tor;
mod util;

const PORT: u16 = 46789;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let dirs = directories::ProjectDirs::from("org", "judica", "tor-chat").unwrap();

    let data_dir: PathBuf = dirs.data_dir().into();
    let dir = tokio::fs::create_dir(&data_dir).await;
    match dir.as_ref().map_err(std::io::Error::kind) {
        Err(std::io::ErrorKind::AlreadyExists) => (),
        e => dir?,
    };
    let mut chat_db_file = data_dir.clone();
    chat_db_file.push("chat.sqlite3");
    let mdb = MsgDB::new(Arc::new(tokio::sync::Mutex::new(
        Connection::open(chat_db_file).unwrap(),
    )));
    mdb.get_handle().await.setup_tables();

    let jh2 = attestations::server::run(PORT, mdb.clone()).await;
    let jh = tor::start(data_dir.clone(), PORT);
    let client = peer_services::client_fetching(mdb.clone());

    let (_, _, _) = tokio::join!(jh2, jh, client);
    Ok(())
}
