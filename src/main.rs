use axum::{
    http::StatusCode,
    http::{self, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use ruma_serde::Base64;
use ruma_signatures::Ed25519KeyPair;
use sapio_bitcoin::hashes::Hash;
use sapio_bitcoin::{hashes::hex::ToHex, util::key};
use serde::{Deserialize, Serialize};
use sqlite::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::SystemTime;
use std::{default, net::SocketAddr};
use tokio::sync::Mutex;

const PORT: u16 = 46789;
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

fn setup_db(db: &sqlite::Connection) {
    db.execute(
            "
            CREATE TABLE IF NOT EXISTS user (userid INTEGER PRIMARY KEY, nickname TEXT , key TEXT UNIQUE);
            CREATE TABLE IF NOT EXISTS messages
                (mid INTEGER PRIMARY KEY,
                    body TEXT,
                    channel_id TEXT,
                    user INTEGER,
                    received_time INTEGER,
                    sent_time INTEGER,
                    FOREIGN KEY(user) references user(userid),
                    UNIQUE(sent_time, body, channel_id, user)
                );
            PRAGMA journal_mode=WAL;
            ",
        )
        .unwrap();
}

async fn post_message(
    Extension(db): Extension<Arc<Mutex<sqlite::Connection>>>,
    Json(envelope): Json<Envelope>,
) -> Result<(Response<()>, Json<MessageResponse>), (StatusCode, &'static str)> {
    if envelope.channel.len() > 128 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Channel ID Longer than 128 Characters",
        ));
    }
    tracing::debug!("recieved: {:?}", envelope);
    let userid = {
        let mut pkmap2 = ruma_signatures::PublicKeyMap::new();
        let keyhash = sapio_bitcoin::hashes::sha256::Hash::hash(envelope.key.as_bytes());
        let key = Base64::<ruma_serde::base64::Standard>::new(envelope.key.as_bytes().to_vec());
        let hex_key = keyhash.to_hex();
        pkmap2.insert(
            hex_key.clone(),
            [("ed25519:1".to_owned(), key)].into_iter().collect(),
        );
        let userid = {
            let locked = db.lock().await;
            let mut stmt = locked
                .prepare("SELECT * FROM user WHERE key = ? LIMIT 1")
                .unwrap()
                .into_cursor();
            stmt.bind(&[Value::String(hex_key)]).unwrap();
            let row = stmt
                .next()
                .map_err(|_| (StatusCode::BAD_REQUEST, "No User Found"))?
                .ok_or((StatusCode::BAD_REQUEST, "No User Found"))?;
            tracing::debug!("Found user!");
            row[0].clone()
        };

        {
            let reserialized = serde_json::to_value(envelope.clone())
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;
            let signed = serde_json::from_value(reserialized)
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?;
            ruma_signatures::verify_json(&pkmap2, &signed)
                .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid Signatures"))?;
            tracing::debug!("verified signatures");
        }
        userid
    };
    let r = match envelope.msg {
        InnerMessage::Ping(data) => Json(MessageResponse::Pong(data)),
        InnerMessage::Data(data) => {
            {
                let locked = db.lock().await;
                let mut stmt = locked
                                            .prepare("
                                            INSERT INTO messages (body, channel_id, user, sent_time, received_time) VALUES (?, ?, ?, ?, ?)
                                            ")
                                            .unwrap();
                stmt.bind(1, &Value::String(data)).unwrap();
                stmt.bind(2, &Value::String(envelope.channel)).unwrap();
                stmt.bind(3, &userid).unwrap();
                stmt.bind(4, &Value::Integer(envelope.sent_time_ms as i64))
                    .unwrap();
                stmt.bind(
                    5,
                    &Value::Integer(
                        SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .expect("System Time OK")
                            .as_millis() as i64,
                    ),
                )
                .unwrap();

                loop {
                    if stmt
                        .next()
                        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, ""))?
                        == sqlite::State::Done
                    {
                        break;
                    }
                }
            }
            Json(MessageResponse::None)
        }
    };
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        r,
    ))
}
async fn chat_server(
    db: std::sync::Arc<tokio::sync::Mutex<sqlite::Connection>>,
) -> tokio::task::JoinHandle<()> {
    let db = db.clone();

    return tokio::spawn(async {
        // build our application with a route
        let app = Router::new()
            // `POST /msg` goes to `msg`
            .route("/msg", post(post_message))
            .layer(Extension(db));

        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        let addr = SocketAddr::from(([127, 0, 0, 1], PORT));
        tracing::debug!("listening on {}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
}
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
    let mut db = sqlite::open(chat_db_file).unwrap();
    setup_db(&mut db);

    let db = Arc::new(tokio::sync::Mutex::new(db));
    let jh2 = chat_server(db.clone()).await;
    let jh = start_tor(data_dir.clone());

    let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:19050")?;
    let client = reqwest::Client::builder().proxy(proxy).build()?;
    let url = "n6e4vcd6lmznthfrmwa66rghyabd3p2z2rreewearcqgag3hoxktulid.onion";
    let raw_keypair = Ed25519KeyPair::generate()?;
    let keypair = Ed25519KeyPair::from_der(&raw_keypair, "1".into())?;

    let hex_key = {
        let locked = db.lock().await;
        let mut stmt = locked.prepare("INSERT INTO user (nickname, key) VALUES (?, ?)")?;
        stmt.bind(1, &Value::String("test_user".into()))?;
        let keyhash = sapio_bitcoin::hashes::sha256::Hash::hash(keypair.public_key());
        let hex_key = keyhash.to_hex();
        stmt.bind(2, &Value::String(hex_key.clone()))?;
        loop {
            if stmt.next()? == sqlite::State::Done {
                break;
            }
        }
        hex_key
    };
    loop {
        tracing::debug!("Waiting to send message...");
        tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
        tracing::debug!("Sending message...");
        let ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64;
        let msg = Envelope {
            msg: InnerMessage::Ping("hi".into()),
            channel: "hello".into(),
            key: ed25519_dalek::PublicKey::from_bytes(keypair.public_key())?,
            sent_time_ms: ms,
            signatures: Default::default(),
        };
        let mut object = ruma_serde::to_canonical_value(msg)?;
        object
            .as_object_mut()
            .map(|m| ruma_signatures::sign_json(&hex_key, &keypair, m));
        let resp = client
            .post(format!("http://{}:46789/msg", url))
            .json(&object)
            .send()
            .await?
            .bytes()
            .await?;
        tracing::debug!("Response: {:?}", resp);
        //        let msg: MessageResponse = serde_json::from_slice(&resp[..])?;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum InnerMessage {
    Ping(String),
    Data(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Envelope {
    msg: InnerMessage,
    channel: String,
    key: ed25519_dalek::PublicKey,
    sent_time_ms: u64,
    #[serde(default)]
    signatures: ruma_signatures::PublicKeyMap,
}

#[derive(Serialize, Deserialize, Debug)]
enum MessageResponse {
    Pong(String),
    None,
}
