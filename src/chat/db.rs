use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
    sync::Arc,
    time::SystemTime,
};

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
        sha256, Hash,
    },
    secp256k1::{Secp256k1, SecretKey, Signing},
    KeyPair, PublicKey, XOnlyPublicKey,
};
use tokio::sync::{Mutex, MutexGuard};

use crate::util::{self, now};

use super::{
    messages::{Authenticated, Envelope, Header, InnerMessage, SigningError, Unsigned},
    nonce::{PrecomittedNonce, PrecomittedPublicNonce},
};

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
        self.0.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS users (user_id INTEGER PRIMARY KEY, nickname TEXT , key TEXT UNIQUE);

            CREATE TABLE IF NOT EXISTS messages
                (message_id INTEGER PRIMARY KEY,
                    body TEXT NOT NULL,
                    user_id INTEGER NOT NULL,
                    received_time INTEGER NOT NULL,
                    FOREIGN KEY(user_id) references users(user_id),
                    UNIQUE(received_time, body, user_id)
                );

            ALTER TABLE messages
            ADD COLUMN height INTEGER NOT NULL
            as (json_extract(body, '$.header.height'));

            ALTER TABLE messages
            ADD COLUMN sent_time INTEGER NOT NULL
            as (json_extract(body, '$.header.sent_time_ms'));
                
            CREATE INDEX messages_json_indexes on messages(height, sent_time);


            CREATE TABLE IF NOT EXISTS hidden_services (service_id INTEGER PRIMARY KEY, service_url TEXT UNIQUE);

            CREATE TABLE IF NOT EXISTS private_keys
                (key_id INTEGER PRIMARY KEY,
                    public_key TEXT UNIQUE,
                    private_key TEXT UNIQUE);
            
            CREATE TABLE IF NOT EXISTS message_nonces (
                nonce_id INTEGER PRIMARY KEY,
                key_id INTEGER,
                private_key TEXT,
                public_key TEXT,
                FOREIGN KEY(key_id) REFERENCES private_keys(key_id),
                UNIQUE(key_id, private_key, public_key)
            );

            PRAGMA journal_mode=WAL;
            "
        )
        .unwrap();
    }

    pub fn fresh_nonce_for<C: Signing>(
        &self,
        secp: &Secp256k1<C>,
        key: XOnlyPublicKey,
    ) -> Result<PrecomittedPublicNonce, rusqlite::Error> {
        let nonce = PrecomittedNonce::new(secp);
        let pk_nonce = self.save_nonce(nonce, secp, key)?;
        Ok(pk_nonce)
    }

    fn save_nonce<C: Signing>(
        &self,
        nonce: PrecomittedNonce,
        secp: &Secp256k1<C>,
        key: XOnlyPublicKey,
    ) -> Result<PrecomittedPublicNonce, rusqlite::Error> {
        let pk_nonce = nonce.get_public(secp);
        let mut stmt = self.0
                                .prepare("
                                            INSERT INTO message_nonces (key_id, public_key, private_key) 
                                            VALUES (
                                                (SELECT key_id FROM private_keys WHERE public_key = ?),
                                                ?,
                                                ?
                                            )
                                            ")?;
        stmt.insert(rusqlite::params![
            key.to_hex(),
            pk_nonce.0.to_hex(),
            nonce.0.secret_bytes().to_hex(),
        ])?;
        Ok(pk_nonce)
    }
    pub fn find_nonce_secret(
        &self,
        nonce: PrecomittedPublicNonce,
    ) -> Result<PrecomittedNonce, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT (private_key) FROM message_nonces where public_key = ?")?;
        stmt.query_row([nonce.0.to_hex()], |r| r.get::<_, PrecomittedNonce>(0))
    }
    pub fn create_envelope<C: Signing>(
        &self,
        msg: InnerMessage,
        keypair: &KeyPair,
        secp: &Secp256k1<C>,
    ) -> Result<Result<Envelope, SigningError>, rusqlite::Error> {
        let key: XOnlyPublicKey = keypair.x_only_public_key().0;
        // Side effect free...
        let tips = self.get_tips()?;
        let my_tip = self.get_tip_for_user(key)?;
        let sent_time_ms = util::now().ok_or("Unknown Time").expect("Time is Known");
        let secret = self.find_nonce_secret(my_tip.header.next_nonce)?;
        // Has side effects!
        let next_nonce = self.fresh_nonce_for(secp, key)?;
        let mut msg = Envelope {
            header: Header {
                height: my_tip.header.height + 1,
                prev_msg: my_tip.canonicalized_hash().unwrap(),
                tips: tips
                    .iter()
                    .map(|tip| {
                        let h = tip.clone().canonicalized_hash()?;
                        Some((tip.header.key, tip.header.height, h))
                    })
                    .flatten()
                    .collect(),
                next_nonce,
                key,
                sent_time_ms,
                unsigned: Unsigned {
                    signature: Default::default(),
                },
            },
            msg,
        };
        Ok(msg.sign_with(keypair, secp, secret).map(move |_| msg))
    }

    pub fn get_message_at_height_for_user(
        &self,
        key: XOnlyPublicKey,
        height: u64,
    ) -> Result<Envelope, rusqlite::Error> {
        let mut stmt = self.0.prepare("SELECT messages.body  FROM messages WHERE user_id = (SELECT user_id from users where key = ?) AND height = ?")?;
        stmt.query_row(params![key.to_hex(), height], |r| r.get(0))
    }
    pub fn get_tip_for_user(&self, key: XOnlyPublicKey) -> Result<Envelope, rusqlite::Error> {
        let mut stmt = self.0.prepare(
            "SELECT m.body
            FROM messages m
            INNER JOIN users u ON m.user_id = u.user_id
            WHERE m.user_id = (SELECT user_id  FROM users where key = ?)
            ORDER BY m.height DESC
            LIMIT 1
            ",
        )?;
        stmt.query_row([key.to_hex()], |r| r.get(0))
    }
    pub fn get_tips(&self) -> Result<Vec<Envelope>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT body, max(height)  FROM   messages GROUP BY user_id")?;
        let rows = stmt.query([])?;
        let vs: Vec<Envelope> = rows.map(|r| r.get::<_, Envelope>(0)).collect()?;
        Ok(vs)
    }
    pub fn load_all_messages(
        &self,
        key: &sapio_bitcoin::secp256k1::XOnlyPublicKey,
    ) -> Result<Result<Vec<Envelope>, (Envelope, Envelope)>, rusqlite::Error> {
        let mut stmt = self.0.prepare(
            "SELECT (messages.body)
        FROM messages
        INNER JOIN users ON messages.user_id = users.user_id
        WHERE users.key = ?
        ORDER BY messages.height ASC;
        ",
        )?;
        let rows = stmt.query(params![key.to_hex()])?;
        let vs: Vec<Envelope> = rows.map(|r| r.get(0)).collect()?;
        let mut prev = sha256::Hash::hash(&[]);
        let mut prev_height = 0;
        for v in vs.windows(2) {
            if v[0].clone().canonicalized_hash().unwrap() != v[1].header.prev_msg
                || v[0].header.height + 1 != v[1].header.height
                || Some(v[0].header.next_nonce) != v[1].extract_used_nonce()
            {
                return Ok(Err((v[0].clone(), v[1].clone())));
            }
        }
        Ok(Ok(vs))
    }
    pub fn get_keymap(&self) -> Result<BTreeMap<PublicKey, SecretKey>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT (public_key, private_key) FROM private_keys")?;
        let rows = stmt.query([])?;
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
    pub fn try_insert_authenticated_envelope(
        &self,
        data: Authenticated<Envelope>,
    ) -> Result<(), rusqlite::Error> {
        let data = data.inner();
        let mut stmt = self.0.prepare(
            "
                                            INSERT INTO messages (body, user_id, received_time)
                                            VALUES (?, (SELECT user_id FROM users WHERE key = ?), ?)
                                            ",
        )?;
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("System Time OK")
            .as_millis() as i64;
        stmt.insert(rusqlite::params![data, data.header.key.to_hex(), time])?;
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

    pub fn insert_user_by_genesis_envelope(
        &self,
        nickname: String,
        envelope: Authenticated<Envelope>,
    ) -> Result<String, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("INSERT INTO users (nickname, key) VALUES (?, ?)")?;
        let hex_key = envelope.inner_ref().header.key.to_hex();
        stmt.insert(params![nickname, hex_key])?;
        self.try_insert_authenticated_envelope(envelope)?;
        Ok(hex_key)
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use rusqlite::Connection;
    use sapio_bitcoin::psbt::raw::Key;
    use sapio_bitcoin::secp256k1::{rand, All};

    use super::MsgDB;
    use super::*;

    #[tokio::test]
    async fn test_setup_db() {
        setup_db().await;
    }

    #[tokio::test]
    async fn test_add_user() {
        let conn = setup_db().await;
        let secp = Secp256k1::new();
        let test_user = "TestUser".into();
        make_test_user(&secp, &conn, test_user).await;
    }
    #[tokio::test]
    async fn test_envelope_creation() {
        let conn = setup_db().await;
        let secp = Secp256k1::new();
        let test_user = "TestUser".into();
        let kp = make_test_user(&secp, &conn, test_user).await;
        let handle = conn.get_handle().await;
        let envelope_1 = handle
            .create_envelope(InnerMessage::Ping(10), &kp, &secp)
            .unwrap()
            .unwrap();
        let envelope_1 = envelope_1.clone().self_authenticate(&secp).unwrap();
        handle
            .try_insert_authenticated_envelope(envelope_1.clone())
            .unwrap();

        let tips = handle.get_tips().unwrap();
        assert_eq!(tips.len(), 1);
        assert_eq!(&tips[0], envelope_1.inner_ref());
        let my_tip = handle.get_tip_for_user(kp.x_only_public_key().0).unwrap();
        assert_eq!(&my_tip, envelope_1.inner_ref());

        let envelope_2 = handle
            .create_envelope(InnerMessage::Ping(10), &kp, &secp)
            .unwrap()
            .unwrap();
        let envelope_2 = envelope_2.clone().self_authenticate(&secp).unwrap();
        handle
            .try_insert_authenticated_envelope(envelope_2.clone())
            .unwrap();
        let tips = handle.get_tips().unwrap();
        assert_eq!(tips.len(), 1);
        assert_eq!(&tips[0], envelope_2.inner_ref());
        let my_tip = handle.get_tip_for_user(kp.x_only_public_key().0).unwrap();
        assert_eq!(&my_tip, envelope_2.inner_ref());
    }

    async fn make_test_user(secp: &Secp256k1<All>, conn: &MsgDB, name: String) -> KeyPair {
        let handle = conn.get_handle().await;
        let mut rng = rand::thread_rng();
        let (sk, pk) = secp.generate_keypair(&mut rng);
        let key = pk.x_only_public_key().0;
        let nonce = PrecomittedNonce::new(secp);
        let kp = KeyPair::from_secret_key(secp, &sk);
        let mut genesis = Envelope {
            header: Header {
                key,
                next_nonce: nonce.get_public(secp),
                prev_msg: sha256::Hash::hash(&[]),
                tips: vec![],
                height: 0,
                sent_time_ms: util::now().unwrap(),
                unsigned: Unsigned { signature: None },
            },
            msg: InnerMessage::Ping(0),
        };
        genesis
            .sign_with(&kp, secp, PrecomittedNonce::new(secp))
            .unwrap();
        let genesis = genesis.self_authenticate(secp).unwrap();
        handle
            .insert_user_by_genesis_envelope(name, genesis)
            .unwrap();
        handle.save_nonce(nonce, secp, key).unwrap();
        kp
    }

    async fn setup_db() -> MsgDB {
        let conn = MsgDB::new(Arc::new(Mutex::new(Connection::open_in_memory().unwrap())));
        conn.get_handle().await.ensure_created();
        conn
    }
    #[tokio::test]
    async fn test_tables() {
        let mut conn = setup_db().await;
        let handle = conn.get_handle().await;
        let mut it = handle
            .0
            .prepare(
                "SELECT name FROM sqlite_schema
        WHERE type='table'
        ORDER BY name;
        ",
            )
            .unwrap();
        let vit: Vec<_> = it
            .query(params![])
            .unwrap()
            .map(|r| r.get::<_, String>(0))
            .collect()
            .unwrap();
        assert_eq!(
            vec![
                "hidden_services",
                "message_nonces",
                "messages",
                "private_keys",
                "users"
            ],
            vit
        )
    }
}
