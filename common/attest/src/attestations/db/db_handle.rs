use crate::util;

use super::sql_serializers::{self, PK};
use attest_messages::Authenticated;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::Envelope;
use attest_messages::Header;
use attest_messages::SigningError;
use attest_messages::Unsigned;
use attest_messages::nonce::PrecomittedNonce;
use attest_messages::nonce::PrecomittedPublicNonce;
use fallible_iterator::FallibleIterator;
use rusqlite::params;
use rusqlite::Connection;
use sapio_bitcoin::{
    hashes::{hex::ToHex, sha256, Hash},
    secp256k1::{Secp256k1, SecretKey, Signing},
    KeyPair, XOnlyPublicKey,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use tokio::sync::MutexGuard;

pub struct MsgDBHandle<'a>(pub MutexGuard<'a, Connection>);

pub enum ConsistentMessages {
    AllMessagesNotReady,
}

impl<'a> MsgDBHandle<'a> {
    /// Creates all the required tables for the application.
    /// Safe to call multiple times
    pub fn setup_tables(&mut self) {
        self.0
            .execute_batch(include_str!("sql/create_tables.sql"))
            .unwrap();
    }

    /// Creates a new random nonce and saves it for the given user.
    pub fn generate_fresh_nonce_for_user_by_key<C: Signing>(
        &self,
        secp: &Secp256k1<C>,
        key: XOnlyPublicKey,
    ) -> Result<PrecomittedPublicNonce, rusqlite::Error> {
        let nonce = PrecomittedNonce::new(secp);
        let pk_nonce = self.save_nonce_for_user_by_key(nonce, secp, key)?;
        Ok(pk_nonce)
    }
    /// Saves an arbitrary nonce for the given user.
    pub(crate) fn save_nonce_for_user_by_key<C: Signing>(
        &self,
        nonce: PrecomittedNonce,
        secp: &Secp256k1<C>,
        key: XOnlyPublicKey,
    ) -> Result<PrecomittedPublicNonce, rusqlite::Error> {
        let pk_nonce = nonce.get_public(secp);
        let mut stmt = self.0.prepare(include_str!("sql/insert_nonce.sql"))?;
        stmt.insert(rusqlite::params![key.to_hex(), pk_nonce, nonce,])?;
        Ok(pk_nonce)
    }
    /// Returns the secret nonce for a given public nonce
    pub fn get_secret_for_public_nonce(
        &self,
        nonce: PrecomittedPublicNonce,
    ) -> Result<PrecomittedNonce, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT (private_key) FROM message_nonces where public_key = ?")?;
        stmt.query_row([nonce], |r| r.get::<_, PrecomittedNonce>(0))
    }

    /// given an arbitrary inner message, generates an envelope and signs it.
    ///
    /// Calling multiple times with a given nonce would result in nonce reuse.
    pub fn wrap_message_in_envelope_for_user_by_key<C: Signing>(
        &self,
        msg: Value,
        keypair: &KeyPair,
        secp: &Secp256k1<C>,
    ) -> Result<Result<Envelope, SigningError>, rusqlite::Error> {
        let key: XOnlyPublicKey = keypair.x_only_public_key().0;
        // Side effect free...
        let tips = self.get_tips_for_all_users()?;
        let my_tip = self.get_tip_for_user_by_key(key)?;
        let sent_time_ms = util::now();
        let secret = self.get_secret_for_public_nonce(my_tip.header.next_nonce)?;
        // Has side effects!
        let next_nonce = self.generate_fresh_nonce_for_user_by_key(secp, key)?;
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
                // TODO: Fetch from server.
                checkpoints: Default::default(),
            },
            msg,
        };
        Ok(msg.sign_with(keypair, secp, secret).map(move |_| msg))
    }

    /// Returns the message at a given height for a key
    pub fn get_message_at_height_for_user(
        &self,
        key: XOnlyPublicKey,
        height: u64,
    ) -> Result<Envelope, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare(include_str!("sql/get_message_by_height_and_user.sql"))?;
        stmt.query_row(params![key.to_hex(), height], |r| r.get(0))
    }
    /// finds the most recent message for a user by their key
    pub fn get_tip_for_user_by_key(
        &self,
        key: XOnlyPublicKey,
    ) -> Result<Envelope, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare(include_str!("sql/message_tips_by_user.sql"))?;
        stmt.query_row([key.to_hex()], |r| r.get(0))
    }

    /// finds the most recent message only for messages where we know the key
    pub fn get_tip_for_known_keys(&self) -> Result<Vec<Envelope>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare(include_str!("sql/get_tips_for_known_keys.sql"))?;
        let rows = stmt.query([])?;
        let vs: Vec<Envelope> = rows.map(|r| r.get::<_, Envelope>(0)).collect()?;
        Ok(vs)
    }

    /// finds the most recent message only for messages where we know the key
    pub fn get_all_messages(
        &self,
    ) -> Result<Result<HashMap<XOnlyPublicKey, Vec<Envelope>>, ConsistentMessages>, rusqlite::Error>
    {
        let mut stmt = self.0.prepare(include_str!("sql/get_all_messages.sql"))?;
        let rows = stmt.query([])?;
        let mut vs = rows
            .map(|r| r.get::<_, Envelope>(0))
            .fold(HashMap::new(), |mut acc, v| {
                acc.entry(v.header.key).or_insert(vec![]).push(v);
                Ok(acc)
            })?;

        for (k, v) in vs.iter_mut() {
            v.sort_unstable_by_key(|k| k.header.height)
        }
        if vs.iter().all(|(k, v)| v[0].header.height == 0)
            && vs.iter().all(|(k, v)| {
                v.windows(2).all(|w| {
                    w[0].header.height + 1 == w[1].header.height
                        && w[1].header.prev_msg == w[0].clone().canonicalized_hash().unwrap()
                })
            })
        {
            Ok(Ok(vs))
        } else {
            Ok(Err(ConsistentMessages::AllMessagesNotReady))
        }
    }

    /// finds a reused nonce
    pub fn get_reused_nonces(
        &self,
    ) -> Result<HashMap<XOnlyPublicKey, Vec<Envelope>>, rusqlite::Error> {
        let mut stmt = self.0.prepare("sql/get_reused_nonces.sql")?;
        let rows = stmt.query([])?;
        let vs = rows
            .map(|r| r.get::<_, Envelope>(0))
            .fold(HashMap::new(), |mut acc, v| {
                acc.entry(v.header.key).or_insert(vec![]).push(v);
                Ok(acc)
            })?;

        Ok(vs)
    }

    /// finds all most recent messages for all users
    pub fn get_tips_for_all_users(&self) -> Result<Vec<Envelope>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT body, max(height)  FROM   messages GROUP BY user_id")?;
        let rows = stmt.query([])?;
        let vs: Vec<Envelope> = rows.map(|r| r.get::<_, Envelope>(0)).collect()?;
        Ok(vs)
    }

    /// loads all the messages from a given user
    pub fn load_all_messages_for_user_by_key(
        &self,
        key: &sapio_bitcoin::secp256k1::XOnlyPublicKey,
    ) -> Result<Result<Vec<Envelope>, (Envelope, Envelope)>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare(include_str!("sql/load_all_messages_by_key.sql"))?;
        let rows = stmt.query(params![key.to_hex()])?;
        let vs: Vec<Envelope> = rows.map(|r| r.get(0)).collect()?;
        let _prev = sha256::Hash::hash(&[]);
        let _prev_height = 0;
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

    /// build a keymap for all known keypairs.
    pub fn get_keymap(&self) -> Result<BTreeMap<XOnlyPublicKey, SecretKey>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT (public_key, private_key) FROM private_keys")?;
        let rows = stmt.query([])?;
        rows.map(|r| {
            Ok((
                r.get::<_, sql_serializers::PK>(0)?.0,
                r.get::<_, sql_serializers::SK>(1)?.0,
            ))
        })
        .collect()
    }

    /// adds a hidden service to our connection list
    pub fn insert_hidden_service(&self, s: String, port: u16) -> Result<(), rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("INSERT INTO hidden_services (service_url, port) VALUES (?,?)")?;
        stmt.insert(rusqlite::params![s, port])?;
        Ok(())
    }

    /// get all added hidden services
    pub fn get_all_hidden_services(&self) -> Result<Vec<(String, u16)>, rusqlite::Error> {
        let mut stmt = self.0.prepare("SELECT service_url FROM hidden_services")?;
        let results = stmt
            .query([])?
            .map(|r| Ok((r.get::<_, String>(0)?, r.get(1)?)))
            .collect()?;
        Ok(results)
    }

    /// saves a keypair to our keyset
    pub fn save_keypair(&self, kp: KeyPair) -> Result<(), rusqlite::Error> {
        let mut stmt = self.0
                                .prepare("
                                            INSERT INTO private_keys (public_key, private_key) VALUES (?, ?)
                                            ")?;
        stmt.insert(rusqlite::params![
            kp.x_only_public_key().0.to_hex(),
            kp.secret_bytes().to_hex()
        ])?;
        Ok(())
    }

    pub fn message_exists(&self, hash: &sha256::Hash) -> Result<bool, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT EXISTS(SELECT 1 FROM messages WHERE hash = ?)")?;
        stmt.exists([hash.to_hex()])
    }

    pub fn messages_by_hash<'i, I>(&self, hashes: I) -> Result<Vec<Envelope>, rusqlite::Error>
    where
        I: Iterator<Item = &'i CanonicalEnvelopeHash>,
    {
        let mut stmt = self.0.prepare("SELECT body FROM messages WHERE hash = ?")?;
        let r: Result<Vec<_>, _> = hashes
            .map(|hash| stmt.query_row([hash], |r| r.get::<_, Envelope>(0)))
            .collect();
        r
    }
    pub fn message_not_exists_it<'i, I>(
        &self,
        hashes: I,
    ) -> Result<Vec<CanonicalEnvelopeHash>, rusqlite::Error>
    where
        I: Iterator<Item = &'i CanonicalEnvelopeHash>,
    {
        let mut stmt = self
            .0
            .prepare("SELECT EXISTS(SELECT 1 FROM messages WHERE hash = ?)")?;
        hashes
            .filter_map(|hash| match stmt.exists([hash]) {
                Ok(true) => None,
                Ok(false) => Some(Ok(*hash)),
                Err(x) => Some(Err(x)),
            })
            .collect()
    }

    /// attempts to put an authenticated envelope in the DB
    ///
    /// Will fail if the key is not registered.
    pub fn try_insert_authenticated_envelope(
        &self,
        data: Authenticated<Envelope>,
    ) -> Result<(), rusqlite::Error> {
        let data = data.inner();
        let mut stmt = self.0.prepare(include_str!("sql/insert_envelope.sql"))?;
        let time = util::now();

        stmt.insert(rusqlite::params![
            data,
            data.clone()
                .canonicalized_hash()
                .expect("Hashing should always succeed?"),
            data.header.key.to_hex(),
            time
        ])?;
        Ok(())
    }

    /// finds a user by key
    pub fn locate_user(
        &self,
        key: &sapio_bitcoin::secp256k1::XOnlyPublicKey,
    ) -> Result<i64, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT user_id FROM users WHERE key = ? LIMIT 1")?;
        stmt.query_row([key.to_hex()], |row| row.get(0))
    }

    /// creates a new user from a genesis envelope
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
