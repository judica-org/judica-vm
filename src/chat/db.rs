use std::{sync::Arc, time::SystemTime};

use ruma_signatures::Ed25519KeyPair;
use rusqlite::Connection;
use sapio_bitcoin::{
    hashes::{
        hex::{self, ToHex},
        Hash,
    },
    XOnlyPublicKey,
};
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
            ",[]
        )
        .unwrap();
    }

    pub fn insert_msg(
        &self,
        data: String,
        channel: String,
        sent_time_ms: u64,
        userid: i64,
    ) -> Result<(), rusqlite::Error> {
        let mut stmt = self.0
                                .prepare("
                                            INSERT INTO messages (body, channel_id, user, sent_time, received_time) VALUES (?, ?, ?, ?, ?)
                                            ")?;
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("System Time OK")
            .as_millis() as i64;
        stmt.insert(rusqlite::params![data, channel, userid, sent_time_ms, time])?;
        Ok(())
    }

    pub fn locate_user(
        &self,
        key: &sapio_bitcoin::secp256k1::XOnlyPublicKey,
    ) -> Result<i64, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT userid FROM user WHERE key = ? LIMIT 1")?;
        stmt.query_row([key.to_hex()], |row| row.get(0))
    }

    pub fn insert_user(
        &self,
        key: &XOnlyPublicKey,
        nickname: String,
    ) -> Result<String, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("INSERT INTO user (nickname, key) VALUES (?, ?)")?;
        let hex_key = key.to_hex();
        stmt.insert([&nickname, &hex_key])?;
        Ok(hex_key)
    }
}
