#![feature(async_closure)]
use libtor::{HiddenServiceAuthType, HiddenServiceVersion, Tor, TorAddress, TorFlag};
use tokio::net;
const PORT: u16 = 46789;
use ruma_serde::Base64;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::hashes::Hash;
use sqlite::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;
fn start_tor(mut buf: PathBuf) -> JoinHandle<Result<u8, libtor::Error>> {
    buf.push("onion");
    let mut tor = Tor::new();
    tor.flag(TorFlag::DataDirectory(buf.to_str().unwrap().into()));

    buf.push("chatserver");
    tor.flag(TorFlag::SocksPort(19050))
        .flag(TorFlag::HiddenServiceDir(buf.to_str().unwrap().into()))
        .flag(TorFlag::HiddenServiceVersion(HiddenServiceVersion::V3))
        .flag(TorFlag::HiddenServicePort(
            TorAddress::Port(PORT),
            None.into(),
        ))
        .start_background()
}

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn setup_db(db: &sqlite::Connection) {
    db.execute(
            "
            CREATE TABLE IF NOT EXISTS user (userid INTEGER PRIMARY KEY , nickname TEXT , key TEXT UNIQUE);
            CREATE TABLE IF NOT EXISTS messages (body TEXT, channel_id TEXT, user INTEGER, FOREIGN KEY(user) references user(userid));
            PRAGMA journal_mode=WAL;
            ",
        )
        .unwrap();
}

async fn chat_server(
    db: std::sync::Arc<tokio::sync::Mutex<sqlite::Connection>>,
) -> tokio::task::JoinHandle<()> {
    let db = db.clone();
    tokio::spawn(async move {
        async || -> Result<(), Box<dyn std::error::Error>> {
            let listener = TcpListener::bind((std::net::Ipv4Addr::new(127, 0, 0, 1), PORT)).await?;
            println!("Listening on {}", PORT);
            loop {
                let (mut socket, _) = listener.accept().await?;
                let db = db.clone();
                tokio::spawn(async move {
                    let (read, mut write) = tokio::io::split(socket);
                    let br = tokio::io::BufReader::new(read);
                    let mut reader = tokio::io::AsyncBufReadExt::lines(br);
                    // In a loop, read data from the socket and write the data back.
                    // Line GET /msg HTTP/1.1
                    // Line accept: */*
                    // Line host: 3m52h7x2od4mr6i45fy7zqjce35oiagefxylc5l3fcj4uf6ccnaqwayd.onion:46789
                    // Line content-length: 4
                    let method = reader.next_line().await;
                    println!("RECV Req: {:?}", method);
                    if method.ok().flatten() != Some("POST /msg HTTP/1.1".into()) {
                        return;
                    }
                    loop {
                        if let Ok(Some(line)) = reader.next_line().await {
                            println!("RECV Header: {}", line);
                            if Some(line) == Some("".into()) {
                                println!("Breaking...");
                                break;
                            }
                        } else {
                            return;
                        }
                    }
                    if let Ok(Some(body)) = reader.next_line().await {
                        println!("RECV Body: {}", body);
                        if let Ok(envelope) = serde_json::from_str::<Envelope>(&body) {
                            if let Ok(signed) =
                                serde_json::from_str::<ruma_serde::CanonicalJsonObject>(&body)
                            {
                                let mut pkmap2 = ruma_signatures::PublicKeyMap::new();
                                let keyhash = sapio_bitcoin::hashes::sha256::Hash::hash(
                                    envelope.key.as_bytes(),
                                );
                                let key = Base64::<ruma_serde::base64::Standard>::new(
                                    envelope.key.as_bytes().to_vec(),
                                );
                                let hex_key = keyhash.to_hex();
                                pkmap2.insert(
                                    hex_key.clone(),
                                    [("ed25519:1".to_owned(), key)].into_iter().collect(),
                                );
                                let row = {
                                    let locked = db.lock().await;
                                    let mut stmt = locked
                                        .prepare("SELECT * FROM user WHERE key = ? LIMIT 1")
                                        .unwrap();
                                    stmt.bind(1, &Value::String(hex_key)).unwrap();
                                    let row = stmt.next();
                                    row
                                };
                                if let Ok(user) = row {
                                    if ruma_signatures::verify_json(&pkmap2, &signed).is_ok() {
                                        // TODO: check signers in DB
                                        println!("RECV Verified: {:?}", signed);
                                        use tokio::io::AsyncWriteExt;
                                        write.write_all("HTTP/1.1 200 OK\r\n".as_bytes()).await;
                                        let b = serde_json::to_string(&MessageResponse::Pong(body))
                                            .unwrap();
                                        write
                                            .write_all(
                                                "Access-Control-Allow-Origin: *\r\n".as_bytes(),
                                            )
                                            .await;
                                        write
                                        .write_all(
                                            "Content-Type: application/json\r\nContent-Length: "
                                                .as_bytes(),
                                        )
                                        .await;
                                        write.write_all(format!("{}", b.len()).as_bytes()).await;
                                        write.write_all("\r\n\r\n".as_bytes()).await;
                                        write.write_all(b.as_bytes()).await;
                                    } else {
                                        println!("Invalid Signatures on Envelope")
                                    }
                                } else {
                                    println!("incompatible, should be unreachavle")
                                }
                            } else {
                                write
                                    .write_all("HTTP/1.1 400 Bad Request\r\n\r\n".as_bytes())
                                    .await;
                            }
                        }
                    }
                });
            }
        }()
        .await;
    })
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dirs = directories::ProjectDirs::from("org", "judica", "tor-chat").unwrap();

    let data_dir: PathBuf = dirs.data_dir().into();
    tokio::fs::create_dir(&data_dir).await;
    let mut chat_db_file = data_dir.clone();
    chat_db_file.push("chat.sqlite3");
    let mut db = sqlite::open(chat_db_file).unwrap();
    setup_db(&mut db);

    let channel = sapio_bitcoin::hashes::sha256::Hash::hash("hello".as_bytes());
    let key = ed25519_dalek::Keypair::generate(&mut rand::rngs::OsRng {});
    println!(
        "{}",
        serde_json::to_string_pretty(&Envelope {
            msg: InnerMessage::Ping("hello".into()),
            channel,
            key: key.public,
            signatures: Default::default()
        })?
    );

    let jh2 = chat_server(Arc::new(tokio::sync::Mutex::new(db))).await;
    let jh = start_tor(data_dir.clone());

    loop {}
    let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:19050")?;
    let client = reqwest::Client::builder().proxy(proxy).build()?;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
        println!("TRYING:");
        let mut msg = serde_json::to_string(&InnerMessage::Ping("hi".into()))?;
        msg.push('\r' as char);
        msg.push('\n' as char);
        let resp = client
            .post("http://3m52h7x2od4mr6i45fy7zqjce35oiagefxylc5l3fcj4uf6ccnaqwayd.onion:46789/msg")
            .body(msg)
            .send()
            .await?
            .bytes()
            .await?;
        println!("{:?}", resp);
        let msg: MessageResponse = serde_json::from_slice(&resp[..])?;
    }
}

use serde::{Deserialize, Serialize};
use serde_derive::*;
#[derive(Serialize, Deserialize, Debug)]
enum InnerMessage {
    Ping(String),
    Data(String),
}

#[derive(Serialize, Deserialize, Debug)]
struct Envelope {
    msg: InnerMessage,
    channel: sapio_bitcoin::hashes::sha256::Hash,
    key: ed25519_dalek::PublicKey,
    #[serde(default)]
    signatures: ruma_signatures::PublicKeyMap,
}

#[derive(Serialize, Deserialize, Debug)]
enum MessageResponse {
    Pong(String),
}
