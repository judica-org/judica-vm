use std::{sync::Arc, time::SystemTime};

use ruma_signatures::Ed25519KeyPair;
use sapio_bitcoin::hashes::{hex::ToHex, Hash};
use sqlite::{Connection, Value};
use tokio::sync::{Mutex, MutexGuard};

#[derive(Clone)]
pub struct MsgDB(Arc<Mutex<Connection>>);
impl MsgDB {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        MsgDB(db)
    }
    pub async fn get_handle<'a>(&'a self) -> MsgDBHandle<'a> {
        MsgDBHandle(self.0.lock().await)
    }
}
pub struct MsgDBHandle<'a>(pub MutexGuard<'a, Connection>);
impl<'a> MsgDBHandle<'a> {
    pub fn ensure_created(&mut self) {
        self.0.execute(
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

    pub fn insert_msg(
        &self,
        data: String,
        channel: String,
        sent_time_ms: u64,
        userid: Value,
    ) -> Result<(), sqlite::Error> {
        let mut stmt = self.0
                                .prepare("
                                            INSERT INTO messages (body, channel_id, user, sent_time, received_time) VALUES (?, ?, ?, ?, ?)
                                            ")?;
        stmt.bind(1, &Value::String(data))?;
        stmt.bind(2, &Value::String(channel))?;
        stmt.bind(3, &userid)?;
        stmt.bind(4, &Value::Integer(sent_time_ms as i64))?;
        stmt.bind(
            5,
            &Value::Integer(
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("System Time OK")
                    .as_millis() as i64,
            ),
        )?;

        loop {
            if stmt.next()? == sqlite::State::Done {
                return Ok(());
            }
        }
    }

    pub fn locate_user(&self, hex_key: String) -> Result<Option<Value>, sqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT * FROM user WHERE key = ? LIMIT 1")?
            .into_cursor();
        stmt.bind(&[Value::String(hex_key)])?;
        let row = stmt.next()?;
        Ok(row.map(|r| r.get(0).cloned()).flatten())
    }

    pub fn insert_user(
        &self,
        keypair: &Ed25519KeyPair,
        nickname: String,
    ) -> Result<String, sqlite::Error> {
        let mut stmt = self
            .0
            .prepare("INSERT INTO user (nickname, key) VALUES (?, ?)")?;
        stmt.bind(1, &Value::String(nickname))?;
        let keyhash = sapio_bitcoin::hashes::sha256::Hash::hash(keypair.public_key());
        let hex_key = keyhash.to_hex();
        stmt.bind(2, &Value::String(hex_key.clone()))?;
        loop {
            if stmt.next()? == sqlite::State::Done {
                break;
            }
        }
        Ok(hex_key)
    }
}
