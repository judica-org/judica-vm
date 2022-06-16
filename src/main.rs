use crate::chat::messages::{Envelope, Header, InnerMessage, Unsigned};
use crate::chat::nonce::PrecomittedNonce;
use chat::db::MsgDB;
use ruma_signatures::Ed25519KeyPair;
use rusqlite::Connection;
use sapio_bitcoin::hashes::{sha256, Hash};
use sapio_bitcoin::secp256k1::{rand, Secp256k1};
use sapio_bitcoin::util::key::KeyPair;
use sapio_bitcoin::{hashes::hex::ToHex, util::key};
use std::error::Error;
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
    let mdb = MsgDB::new(Arc::new(tokio::sync::Mutex::new(
        Connection::open(chat_db_file).unwrap(),
    )));
    mdb.get_handle().await.setup_tables();

    let jh2 = chat::server::run(PORT, mdb.clone()).await;
    let jh = tor::start(data_dir.clone(), PORT);
    let client = client_fetching(mdb.clone());

    let (_, _, _) = tokio::join!(jh2, jh, client);
    Ok(())
}

async fn client_fetching(db: MsgDB) -> Result<(), Box<dyn std::error::Error>> {
    let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:19050")?;
    let client = reqwest::Client::builder().proxy(proxy).build()?;

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        let services = db.get_handle().await.get_all_hidden_services()?;
        let reqs = services.into_iter().map(|url| {
            let client = client.clone();
            async move {
                tracing::debug!("Sending message...");
                let resp : Vec<Envelope> = client
                    .get(format!("http://{}:{}/tips", url, PORT))
                    .send()
                    .await?
                    .json()
                    .await?;
                tracing::debug!("Response: {:?}", resp);
                Ok::<(), Box<dyn Error>>(())
            }
        });
        futures::future::join_all(reqs).await;
    }
}

fn generate_new_user() -> Result<
    (
        Secp256k1<sapio_bitcoin::secp256k1::All>,
        KeyPair,
        PrecomittedNonce,
        Envelope,
    ),
    Box<dyn Error>,
> {
    let secp = Secp256k1::new();
    let keypair: _ = KeyPair::new(&secp, &mut rand::thread_rng());
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
    Ok((secp, keypair, nonce, msg))
}
