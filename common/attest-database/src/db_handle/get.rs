use super::sql_serializers::{self};
use super::{handle_type, ConsistentMessages, MsgDBHandle};
use attest_messages::nonce::PrecomittedNonce;
use attest_messages::nonce::PrecomittedPublicNonce;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::Envelope;
use fallible_iterator::FallibleIterator;
use rusqlite::{named_params, params};
use sapio_bitcoin::{
    hashes::{hex::ToHex, sha256, Hash},
    secp256k1::SecretKey,
    XOnlyPublicKey,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
#[derive(Serialize, Deserialize)]
pub struct PeerInfo {
    pub service_url: String,
    pub port: u16,
    pub fetch_from: bool,
    pub push_to: bool,
    pub allow_unsolicited_tips: bool,
}
impl<'a, T> MsgDBHandle<'a, T>
where
    T: handle_type::Get,
{
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

    /// Returns the message at a given height for a key
    pub fn get_connected_messages_newer_than_envelopes<'v, It>(
        &self,
        envelopes: It,
    ) -> Result<Vec<Envelope>, rusqlite::Error>
    where
        It: Iterator<Item = &'v Envelope>,
    {
        let mut stmt = self.0.prepare(include_str!(
            "sql/get/connected_messages_newer_than_for_genesis.sql"
        ))?;
        let mut res = vec![];
        for envelope in envelopes {
            let rs = stmt
                .query(named_params! {":genesis": envelope.header.genesis, ":height": envelope.header.height})?;
            let mut envs = rs.map(|r| r.get::<_, Envelope>(0)).collect()?;
            res.append(&mut envs);
        }
        Ok(res)
    }
    /// Returns the message at a given height for a key
    pub fn get_message_at_height_for_user(
        &self,
        key: XOnlyPublicKey,
        height: u64,
    ) -> Result<Envelope, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare(include_str!("sql/get/message_by_height_and_user.sql"))?;
        stmt.query_row(params![key.to_hex(), height], |r| r.get(0))
    }

    /// finds the most recent message for a user by their key
    pub fn get_tip_for_user_by_key(
        &self,
        key: XOnlyPublicKey,
    ) -> Result<Envelope, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare(include_str!("sql/get/message_tips_by_user.sql"))?;
        stmt.query_row([key.to_hex()], |r| r.get(0))
    }

    /// finds the most recent message only for messages where we know the key
    pub fn get_tip_for_known_keys(&self) -> Result<Vec<Envelope>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare(include_str!("sql/get/tips_for_known_keys.sql"))?;
        let rows = stmt.query([])?;
        let vs: Vec<Envelope> = rows.map(|r| r.get::<_, Envelope>(0)).collect()?;
        Ok(vs)
    }

    pub fn get_disconnected_tip_for_known_keys(&self) -> Result<Vec<Envelope>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare(include_str!("sql/get/disconnected_tips_for_known_keys.sql"))?;
        let rows = stmt.query([])?;
        let vs: Vec<Envelope> = rows.map(|r| r.get::<_, Envelope>(0)).collect()?;
        Ok(vs)
    }

    #[cfg(test)]
    pub fn drop_message_by_hash(&self, h: CanonicalEnvelopeHash) -> Result<(), rusqlite::Error> {
        use rusqlite::named_params;

        let mut stmt = self.0.prepare("DELETE FROM messages WHERE :hash = hash")?;
        stmt.execute(named_params! {":hash": h})?;
        Ok(())
    }

    /// finds the most recent message only for messages where we know the key
    pub fn get_all_messages_by_key_consistent(
        &self,
        newer: Option<i64>,
    ) -> Result<
        Result<(HashMap<XOnlyPublicKey, Vec<Envelope>>, Option<i64>), ConsistentMessages>,
        rusqlite::Error,
    > {
        let mut stmt = if newer.is_some() {
            self.0.prepare(include_str!("sql/get/all_messages.sql"))?
        } else {
            self.0
                .prepare(include_str!("sql/get/all_messages_after.sql"))?
        };
        let rows = match newer {
            Some(i) => stmt.query([i])?,
            None => stmt.query([])?,
        };
        let (mut vs, newest) = rows
            .map(|r| Ok((r.get::<_, Envelope>(0)?, r.get::<_, i64>(1)?)))
            .fold((HashMap::new(), None), |(mut acc, max_id), (v, id)| {
                acc.entry(v.header.key).or_insert(vec![]).push(v);
                Ok((acc, max_id.max(Some(id))))
            })?;

        for v in vs.values_mut() {
            v.sort_unstable_by_key(|k| k.header.height)
        }
        // TODO: Make this more consistent by being able to drop a pending suffix.
        if vs.values().all(|v| v[0].header.height == 0)
            && vs.values().all(|v| {
                v.windows(2).all(|w| {
                    w[0].header.height + 1 == w[1].header.height
                        && w[1].header.prev_msg == w[0].clone().canonicalized_hash().unwrap()
                })
            })
        {
            Ok(Ok((vs, newest)))
        } else {
            Ok(Err(ConsistentMessages::AllMessagesNotReady))
        }
    }

    pub fn get_all_messages_collect_into_inconsistent(
        &self,
        newer: &mut Option<i64>,
        map: &mut HashMap<CanonicalEnvelopeHash, Envelope>,
    ) -> Result<(), rusqlite::Error> {
        let mut stmt = if newer.is_some() {
            self.0.prepare(include_str!("sql/get/all_messages.sql"))?
        } else {
            self.0
                .prepare(include_str!("sql/get/all_messages_after.sql"))?
        };
        let rows = match newer {
            Some(i) => stmt.query([*i])?,
            None => stmt.query([])?,
        };
        rows.map(|r| Ok((r.get::<_, Envelope>(0)?, r.get::<_, i64>(1)?)))
            .for_each(|(v, id)| {
                map.insert(v.canonicalized_hash_ref().unwrap(), v);
                *newer = (*newer).max(Some(id));
                Ok(())
            })?;
        Ok(())
    }

    /// finds a reused nonce
    pub fn get_reused_nonces(
        &self,
    ) -> Result<HashMap<XOnlyPublicKey, Vec<Envelope>>, rusqlite::Error> {
        let mut stmt = self.0.prepare(include_str!("sql/get/reused_nonces.sql"))?;
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
            .prepare(include_str!("sql/get/all_tips_for_all_users.sql"))?;
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
            .prepare(include_str!("sql/get/all_messages_by_key.sql"))?;
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
            .prepare("SELECT public_key, private_key FROM private_keys")?;
        let rows = stmt.query([])?;
        rows.map(|r| {
            Ok((
                r.get::<_, sql_serializers::PK>(0)?.0,
                r.get::<_, sql_serializers::SK>(1)?.0,
            ))
        })
        .collect()
    }

    /// get all added hidden services
    pub fn get_all_hidden_services(&self) -> Result<Vec<PeerInfo>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT service_url, port, fetch_from, push_to, allow_unsolicited_tips FROM hidden_services")?;
        let results = stmt
            .query([])?
            .map(|r| {
                let service_url = r.get::<_, String>(0)?;
                let port = r.get(1)?;
                let fetch_from = r.get(2)?;
                let push_to = r.get(3)?;
                let allow_unsolicited_tips = r.get(4)?;
                Ok(PeerInfo {
                    service_url,
                    port,
                    fetch_from,
                    push_to,
                    allow_unsolicited_tips,
                })
            })
            .collect()?;
        Ok(results)
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

    /// finds a user by key
    pub fn locate_user(
        &self,
        key: &sapio_bitcoin::secp256k1::XOnlyPublicKey,
    ) -> Result<(i64, String), rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("SELECT user_id, nickname  FROM users WHERE key = ? LIMIT 1")?;
        stmt.query_row([key.to_hex()], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
    }

    pub fn get_all_users(&self) -> Result<Vec<(XOnlyPublicKey, String)>, rusqlite::Error> {
        let mut stmt = self.0.prepare("SELECT key, nickname  FROM users")?;
        let q = stmt.query([])?;

        q.mapped(|row| {
            let xonly_public_key = row.get::<_, sql_serializers::PK>(0)?.0;
            let nickname = row.get::<_, String>(1)?;
            Ok((xonly_public_key, nickname))
        })
        .collect()
    }
}
