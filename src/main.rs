use crate::chat::messages::{Envelope, InnerMessage};
use chat::db::MsgDB;
use ruma_signatures::Ed25519KeyPair;
use sapio_bitcoin::hashes::Hash;
use sapio_bitcoin::secp256k1::{rand, Secp256k1};
use sapio_bitcoin::util::key::KeyPair;
use sapio_bitcoin::{hashes::hex::ToHex, util::key};
use sqlite::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;
const PORT: u16 = 46789;
mod chat;
mod tor;

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
        sqlite::open(chat_db_file).unwrap(),
    )));
    mdb.get_handle().await.ensure_created();

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
        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64;
        let mut msg = Envelope {
            msg: InnerMessage::Ping("hi".into()),
            channel: "hello".into(),
            key: keypair.public_key().x_only_public_key().0,
            sent_time_ms: ms,
            signature: Default::default(),
        };
        msg.sign_with(&keypair, &secp)?;
        let resp = client
            .post(format!("http://{}:46789/msg", url))
            .json(&msg)
            .send()
            .await?
            .bytes()
            .await?;
        tracing::debug!("Response: {:?}", resp);
        //        let msg: MessageResponse = serde_json::from_slice(&resp[..])?;
    }
}
