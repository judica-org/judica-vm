use std::{collections::BTreeMap, str::FromStr, sync::Arc, time::SystemTime};

use fallible_iterator::FallibleIterator;
use ruma_signatures::Ed25519KeyPair;
use rusqlite::{
    params,
    types::{FromSql, FromSqlError},
    Connection,
};
use sapio_bitcoin::{
    hashes::{
        hex::{self, ToHex},
        Hash,
    },
    secp256k1::SecretKey,
    KeyPair, PublicKey, XOnlyPublicKey,
};
use tokio::sync::{Mutex, MutexGuard};

use super::messages::Envelope;

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

struct SK(SecretKey);
struct PK(PublicKey);
impl FromSql for SK {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        SecretKey::from_str(s)
            .map_err(|e| FromSqlError::Other(Box::new(e)))
            .map(SK)
    }
}
impl FromSql for PK {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        PublicKey::from_str(s)
            .map_err(|e| FromSqlError::Other(Box::new(e)))
            .map(PK)
    }
}
pub struct MsgDBHandle<'a>(pub MutexGuard<'a, Connection>);
impl<'a> MsgDBHandle<'a> {
    pub fn ensure_created(&mut self) {
        self.0.execute(
            "
            CREATE TABLE IF NOT EXISTS users (user_id INTEGER PRIMARY KEY, nickname TEXT , key TEXT UNIQUE, initial TEXT);

            CREATE TABLE IF NOT EXISTS messages
                (message_id INTEGER PRIMARY KEY,
                    body TEXT,
                    user INTEGER,
                    received_time INTEGER,
                    sent_time INTEGER,
                    FOREIGN KEY(user) references users(user_id),
                    UNIQUE(sent_time, body, user)
                );

            CREATE TABLE IF NOT EXISTS hidden_services (service_id INTEGER PRIMARY KEY, service_url TEXT UNIQUE);

            CREATE TABLE IF NOT EXISTS private_keys
                (key_id INTEGER PRIMARY KEY,
                    public_key TEXT UNIQUE,
                    private_key TEXT UNIQUE);

            PRAGMA journal_mode=WAL;
            ",[]
        )
        .unwrap();
    }

    pub fn load_all_messages(
        &self,

        key: &sapio_bitcoin::secp256k1::XOnlyPublicKey,
    ) -> Result<Vec<Envelope>, rusqlite::Error> {
        let mut stmt = self.0.prepare(
            "SELECT (messages.body)
        FROM messages
        INNER JOIN users ON messages.user = users.user_id
        WHERE users.key = ?
        ",
        )?;
        let rows = stmt.query(params![key.to_hex()])?;
        let v: Vec<Envelope> = rows.map(|r| r.get(0)).collect()?;
        Ok(v)
    }
    pub fn get_keymap(&self) -> Result<BTreeMap<PublicKey, SecretKey>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT (public_key, private_key) FROM private_keys")?;
        let rows = stmt.query([])?;
        use fallible_iterator::FallibleIterator;
        rows.map(|r| Ok((r.get::<_, PK>(0)?.0, r.get::<_, SK>(1)?.0)))
            .collect()
    }
    pub fn insert_hidden_service(&self, s: String) -> Result<(), rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("INSERT INTO hidden_services (service_url) VALUES (?)")?;
        stmt.insert(rusqlite::params![s])?;
        Ok(())
    }
    pub fn save_keypair(&self, kp: KeyPair) -> Result<(), rusqlite::Error> {
        let mut stmt = self.0
                                .prepare("
                                            INSERT INTO private_keys (public_key, private_key) VALUES (?, ?)
                                            ")?;
        stmt.insert(rusqlite::params![
            kp.public_key().to_hex(),
            kp.secret_bytes().to_hex()
        ])?;
        Ok(())
    }
    pub fn insert_msg(
        &self,
        data: Envelope,
        sent_time_ms: u64,
        user_id: i64,
    ) -> Result<(), rusqlite::Error> {
        let mut stmt = self.0
                                .prepare("
                                            INSERT INTO messages (body, user, sent_time, received_time) VALUES (?, ?, ?, ?, ?)
                                            ")?;
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("System Time OK")
            .as_millis() as i64;
        stmt.insert(rusqlite::params![data, user_id, sent_time_ms, time])?;
        Ok(())
    }

    pub fn locate_user(
        &self,
        key: &sapio_bitcoin::secp256k1::XOnlyPublicKey,
    ) -> Result<i64, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT user_id FROM users WHERE key = ? LIMIT 1")?;
        stmt.query_row([key.to_hex()], |row| row.get(0))
    }

    pub fn insert_user(
        &self,
        key: &XOnlyPublicKey,
        nickname: String,
        initial: Envelope,
    ) -> Result<String, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("INSERT INTO users (nickname, key, initial) VALUES (?, ?, ?)")?;
        let hex_key = key.to_hex();
        let env = serde_json::to_value(initial).map_err(|e| FromSqlError::Other(Box::new(e)))?;
        stmt.insert(params![nickname, hex_key, env])?;
        Ok(hex_key)
    }
}
