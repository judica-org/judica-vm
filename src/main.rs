use crate::chat::messages::{Envelope, Header, InnerMessage, Unsigned};
use crate::chat::nonce::PrecomittedNonce;
use chat::db::MsgDB;
use ruma_signatures::Ed25519KeyPair;
use rusqlite::Connection;
use sapio_bitcoin::hashes::{sha256, Hash};
use sapio_bitcoin::secp256k1::{rand, Secp256k1};
use sapio_bitcoin::util::key::KeyPair;
use sapio_bitcoin::{hashes::hex::ToHex, util::key};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;
mod chat;
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
    let mut mdb = MsgDB::new(Arc::new(tokio::sync::Mutex::new(
        Connection::open(chat_db_file).unwrap(),
    )));
    mdb.get_handle().await.setup_tables();

    let jh2 = chat::server::run(PORT, mdb.clone()).await;
    let jh = tor::start(data_dir.clone(), PORT);

    let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:19050")?;
    let client = reqwest::Client::builder().proxy(proxy).build()?;
    let url = "n6e4vcd6lmznthfrmwa66rghyabd3p2z2rreewearcqgag3hoxktulid.onion";

    let secp = Secp256k1::new();
    let keypair: _ = KeyPair::new(&secp, &mut rand::thread_rng());
    loop {
        tracing::debug!("Waiting to send message...");
        tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
        tracing::debug!("Sending message...");
        let nonce = PrecomittedNonce::new(&secp);
        let sent_time_ms = util::now().ok_or("Unknown Time")?;
        let mut msg = Envelope {
            header: Header {
                height: 0,
                prev_msg: sha256::Hash::hash(&[]),
                tips: Vec::new(),
                next_nonce: nonce.get_public(&secp),
                key: keypair.public_key().x_only_public_key().0,
                sent_time_ms,
                unsigned: Unsigned {
                    signature: Default::default(),
                },
            },
            msg: InnerMessage::Ping(sent_time_ms),
        };
        msg.sign_with(&keypair, &secp, nonce)?;
        let resp = client
            .post(format!("http://{}:46789/msg", url))
            .json(&msg)
            .send()
            .await?
            .bytes()
            .await?;
        tracing::debug!("Response: {:?}", resp);
    }
}
