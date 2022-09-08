use super::sql_serializers::{self};
use super::{handle_type, ChainCommitGroupID, MessageID, MsgDBHandle};
use attest_messages::nonce::PrecomittedNonce;
use attest_messages::nonce::PrecomittedPublicNonce;
use attest_messages::Envelope;
use attest_messages::{Authenticated, CanonicalEnvelopeHash};
use fallible_iterator::FallibleIterator;
use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use rusqlite::types::FromSql;
use rusqlite::{named_params, params};
use sapio_bitcoin::hashes::HashEngine;
use sapio_bitcoin::secp256k1::Message;
use sapio_bitcoin::{
    hashes::{hex::ToHex, sha256, Hash},
    secp256k1::SecretKey,
    XOnlyPublicKey,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::ops::Deref;
use tracing::{debug, trace};
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
            .prepare_cached("SELECT (private_key) FROM message_nonces where public_key = ?")?;
        stmt.query_row([nonce], |r| r.get::<_, PrecomittedNonce>(0))
    }

    /// Returns the message at a given height for a key
    pub fn get_connected_messages_newer_than_envelopes<'v, It>(
        &self,
        envelopes: It,
    ) -> Result<Vec<Authenticated<Envelope>>, rusqlite::Error>
    where
        It: Iterator<Item = &'v Authenticated<Envelope>>,
    {
        let mut stmt = self.0.prepare_cached(include_str!(
            "sql/get/connected_messages_newer_than_for_genesis.sql"
        ))?;
        let mut res = vec![];
        for envelope in envelopes {
            let rs = stmt
                .query(named_params! (":genesis": envelope.get_genesis_hash(), ":height": envelope.header().height()))?;
            let mut envs = rs
                .map(|r| r.get::<_, Authenticated<Envelope>>(0))
                .collect()?;
            res.append(&mut envs);
        }
        Ok(res)
    }
    /// Returns the message at a given height for a key
    pub fn get_message_at_height_for_user(
        &self,
        key: XOnlyPublicKey,
        height: u64,
    ) -> Result<Authenticated<Envelope>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(include_str!("sql/get/message_by_height_and_user.sql"))?;
        stmt.query_row(params![key.to_hex(), height], |r| r.get(0))
    }

    /// finds the most recent message for a user by their key
    pub fn get_tip_for_user_by_key(
        &self,
        key: XOnlyPublicKey,
    ) -> Result<Authenticated<Envelope>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(include_str!("sql/get/message_tips_by_user.sql"))?;
        stmt.query_row([key.to_hex()], |r| r.get(0))
    }

    /// finds the most recent message only for messages where we know the key
    pub fn get_tip_for_known_keys(&self) -> Result<Vec<Authenticated<Envelope>>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(include_str!("sql/get/tips_for_known_keys.sql"))?;
        let rows = stmt.query([])?;
        let vs: Vec<Authenticated<Envelope>> = rows
            .map(|r| r.get::<_, Authenticated<Envelope>>(0))
            .collect()?;
        Ok(vs)
    }

    pub fn get_disconnected_tip_for_known_keys(
        &self,
    ) -> Result<Vec<Authenticated<Envelope>>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(include_str!("sql/get/disconnected_tips_for_known_keys.sql"))?;
        let rows = stmt.query([])?;
        let vs: Vec<Authenticated<Envelope>> = rows
            .map(|r| r.get::<_, Authenticated<Envelope>>(0))
            .collect()?;
        Ok(vs)
    }

    #[cfg(test)]
    pub fn drop_message_by_hash(&self, h: CanonicalEnvelopeHash) -> Result<(), rusqlite::Error> {
        use rusqlite::named_params;

        let mut stmt = self
            .0
            .prepare_cached("DELETE FROM messages WHERE :hash = hash")?;
        stmt.execute(named_params! {":hash": h})?;
        Ok(())
    }

    /// finds the most recent message only for messages where we know the key
    pub fn get_all_connected_messages_collect_into(
        &self,
        newer: &mut Option<i64>,
        map: &mut HashMap<CanonicalEnvelopeHash, Authenticated<Envelope>>,
    ) -> Result<(), rusqlite::Error> {
        let mut stmt = if newer.is_some() {
            self.0
                .prepare_cached(include_str!("sql/get/all_messages_after_connected.sql"))?
        } else {
            self.0
                .prepare_cached(include_str!("sql/get/all_messages_connected.sql"))?
        };
        let rows = match newer {
            Some(i) => stmt.query([*i])?,
            None => stmt.query([])?,
        };
        rows.map(|r| Ok((r.get::<_, Authenticated<Envelope>>(0)?, r.get::<_, i64>(1)?)))
            .for_each(|(v, id)| {
                map.insert(v.canonicalized_hash_ref(), v);
                *newer = (*newer).max(Some(id));
                Ok(())
            })?;
        Ok(())
    }

    pub fn get_all_messages_collect_into_inconsistent<E>(
        &self,
        newer: &mut Option<i64>,
        map: &mut HashMap<CanonicalEnvelopeHash, E>,
    ) -> Result<(), rusqlite::Error>
    where
        E: FromSql + AsRef<Envelope>,
    {
        let mut stmt = if newer.is_some() {
            self.0
                .prepare_cached(include_str!("sql/get/all_messages_after.sql"))?
        } else {
            self.0
                .prepare_cached(include_str!("sql/get/all_messages.sql"))?
        };
        let rows = match newer {
            Some(i) => stmt.query([*i])?,
            None => stmt.query([])?,
        };
        rows.map(|r| Ok((r.get::<_, E>(0)?, r.get::<_, i64>(1)?)))
            .for_each(|(v, id)| {
                map.insert(v.as_ref().canonicalized_hash_ref(), v);
                *newer = (*newer).max(Some(id));
                Ok(())
            })?;
        Ok(())
    }

    /// finds a reused nonce
    pub fn get_reused_nonces(
        &self,
    ) -> Result<HashMap<XOnlyPublicKey, Vec<Authenticated<Envelope>>>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(include_str!("sql/get/reused_nonces.sql"))?;
        let rows = stmt.query([])?;
        let vs = rows.map(|r| r.get::<_, Authenticated<Envelope>>(0)).fold(
            HashMap::new(),
            |mut acc, v| {
                acc.entry(v.header().key()).or_insert(vec![]).push(v);
                Ok(acc)
            },
        )?;

        Ok(vs)
    }

    /// finds all most recent messages for all users
    pub fn get_tips_for_all_users<E>(&self) -> Result<Vec<E>, rusqlite::Error>
    where
        E: AsRef<Envelope> + FromSql + std::fmt::Debug,
    {
        let mut stmt = self
            .0
            .prepare_cached(include_str!("sql/get/all_tips_for_all_users.sql"))?;
        let rows = stmt.query([])?;
        let vs: Vec<E> = rows.map(|r| r.get::<_, E>(0)).collect()?;
        debug!(tips=?vs.iter().map(|e| (e.as_ref().header().height(), e.as_ref().get_genesis_hash())).collect::<Vec<_>>(), "Latest Tips Returned");
        trace!(envelopes=?vs, "Tips Returned");
        Ok(vs)
    }

    pub fn get_all_genesis(&self) -> Result<Vec<Authenticated<Envelope>>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(include_str!("sql/get/all_genesis.sql"))?;
        let rows = stmt.query([])?;
        let vs: Vec<Authenticated<Envelope>> = rows
            .map(|r| r.get::<_, Authenticated<Envelope>>(0))
            .collect()?;
        debug!(tips=?vs.iter().map(|e| (e.header().height(), e.get_genesis_hash())).collect::<Vec<_>>(), "Genesis Tips Returned");
        trace!(envelopes=?vs, "Genesis Tips Returned");
        Ok(vs)
    }

    /// loads all the messages from a given user
    pub fn load_all_messages_for_user_by_key_connected(
        &self,
        key: &sapio_bitcoin::secp256k1::XOnlyPublicKey,
    ) -> Result<Vec<Authenticated<Envelope>>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(include_str!("sql/get/all_messages_by_key_connected.sql"))?;
        let rows = stmt.query(params![key.to_hex()])?;
        let vs: Vec<Authenticated<Envelope>> = rows.map(|r| r.get(0)).collect()?;
        Ok(vs)
    }

    /// build a keymap for all known keypairs.
    pub fn get_keymap(&self) -> Result<BTreeMap<XOnlyPublicKey, SecretKey>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached("SELECT public_key, private_key FROM private_keys")?;
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
            .prepare_cached("SELECT service_url, port, fetch_from, push_to, allow_unsolicited_tips FROM hidden_services")?;
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
            .prepare_cached("SELECT EXISTS(SELECT 1 FROM messages WHERE hash = ?)")?;
        stmt.exists([hash.to_hex()])
    }

    pub fn messages_by_hash<'i, I, E>(&self, hashes: I) -> Result<Vec<E>, rusqlite::Error>
    where
        I: Iterator<Item = &'i CanonicalEnvelopeHash>,
        E: AsRef<Envelope> + FromSql,
    {
        let mut stmt = self
            .0
            .prepare_cached("SELECT body FROM messages WHERE hash = ?")?;
        let r: Result<Vec<_>, _> = hashes
            .map(|hash| stmt.query_row([hash], |r| r.get::<_, E>(0)))
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
            .prepare_cached("SELECT 1 FROM messages WHERE hash = ? LIMIT 1")?;
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
            .prepare_cached("SELECT user_id, nickname  FROM users WHERE key = ? LIMIT 1")?;
        stmt.query_row([key.to_hex()], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
    }

    pub fn get_all_users(&self) -> Result<Vec<(XOnlyPublicKey, String)>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached("SELECT key, nickname  FROM users")?;
        let q = stmt.query([])?;

        q.mapped(|row| {
            let xonly_public_key = row.get::<_, sql_serializers::PK>(0)?.0;
            let nickname = row.get::<_, String>(1)?;
            Ok((xonly_public_key, nickname))
        })
        .collect()
    }
}

impl<'a, T> MsgDBHandle<'a, T>
where
    T: handle_type::Get,
{
    pub fn get_all_chain_commit_groups(
        &self,
    ) -> Result<Vec<(ChainCommitGroupID, String)>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(include_str!("sql/get/all_chain_commit_groups.sql"))?;
        let q = stmt.query([])?;
        q.mapped(|row| {
            let r1 = row.get(0)?;
            let r2 = row.get(1)?;
            Ok((r1, r2))
        })
        .collect()
    }

    pub fn get_all_chain_commit_groups_for_chain(
        &self,
        genesis_hash: CanonicalEnvelopeHash,
    ) -> Result<Vec<(ChainCommitGroupID, String)>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(include_str!(
            "sql/get/all_chain_commit_groups_for_chain.sql"
        ))?;
        let q = stmt.query(named_params! {":genesis_hash": genesis_hash})?;
        q.mapped(|row| {
            let r1 = row.get(0)?;
            let r2 = row.get(1)?;
            Ok((r1, r2))
        })
        .collect()
    }

    pub fn get_all_chain_commit_group_members_for_chain(
        &self,
        genesis_hash: CanonicalEnvelopeHash,
    ) -> Result<Vec<MessageID>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(include_str!(
            "sql/get/all_chain_commit_group_members_for_chain.sql"
        ))?;
        let q = stmt.query(named_params! {":genesis_hash": genesis_hash})?;
        q.mapped(|row| {
            let r1 = row.get(0)?;
            Ok(r1)
        })
        .collect()
    }

    pub fn get_all_chain_commit_group_members_tips_for_chain(
        &self,
        genesis_hash: CanonicalEnvelopeHash,
    ) -> Result<Vec<Authenticated<Envelope>>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(include_str!(
            "sql/get/all_chain_commit_group_members_tips_for_chain.sql"
        ))?;
        let q = stmt.query(named_params! {":genesis_hash": genesis_hash})?;
        q.mapped(|row| {
            let r1 = row.get(0)?;
            Ok(r1)
        })
        .collect()
    }
}

pub fn extract_sk_from_envelopes(
    e1: Authenticated<Envelope>,
    e2: Authenticated<Envelope>,
) -> Option<SecretKey> {
    let mut e1 = e1.inner();
    let mut e2 = e2.inner();
    let nonce = e1.extract_used_nonce()?;
    let key = e1.header().key();
    if key != e2.header().key() {
        return None;
    }
    if nonce != e2.extract_used_nonce()? {
        return None;
    }
    let m1 = e1.signature_digest_mut();
    let m2 = e2.signature_digest_mut();
    let s1 = e1.extract_sig_s()?;
    let s2 = e2.extract_sig_s()?;
    extract_sk(key, m1, m2, &nonce.0.serialize(), &s1, &s2)
}

pub fn extract_sk<M1, M2>(
    key: XOnlyPublicKey,
    m1: M1,
    m2: M2,
    nonce: &[u8; 32],
    s1: &[u8; 32],
    s2: &[u8; 32],
) -> Option<SecretKey>
where
    Message: From<M1> + From<M2>,
{
    // H(tag || tag || R || P || m)
    let mut engine = get_signature_tagged_hash();

    engine.input(&nonce[..]);
    engine.input(&key.serialize()[..]);
    let mut engine2 = engine.clone();
    engine.input(Message::from(m1).as_ref());
    engine2.input(Message::from(m2).as_ref());

    let d1 = sha256::Hash::from_engine(engine);
    let d2 = sha256::Hash::from_engine(engine2);

    //    s1 - s2 / d1 - d2 = p

    let s1 = BigInt::from_bytes_be(Sign::Plus, &s1[..]);
    let s2 = BigInt::from_bytes_be(Sign::Plus, &s2[..]);

    let d1 = BigInt::from_bytes_be(Sign::Plus, &d1[..]);
    let d2 = BigInt::from_bytes_be(Sign::Plus, &d2[..]);
    let divisor = d1 - d2;
    let field = BigInt::from_bytes_be(
        Sign::Plus,
        &[
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 254, 186,
            174, 220, 230, 175, 72, 160, 59, 191, 210, 94, 140, 208, 54, 65, 65,
        ][..],
    );

    let res = divisor.extended_gcd(&field);
    #[cfg(test)]
    {
        let res = res.clone();
        let field = field.clone();
        let divisor = divisor.clone();
        assert_eq!(&res.gcd, &1u32.into());
        assert_eq!((res.x * divisor).mod_floor(&field), 1u32.into());
    }

    let inv = res.x.mod_floor(&field);
    let result = (inv * (s1 - s2)).mod_floor(&field);

    let (s, mut sig_bytes) = result.to_bytes_le();
    assert!(s == Sign::Plus);
    while sig_bytes.len() < 32 {
        sig_bytes.push(0);
    }
    sig_bytes.reverse();
    SecretKey::from_slice(&sig_bytes[..]).ok()
}

fn get_signature_tagged_hash() -> sha256::HashEngine {
    let tag = sha256::Hash::hash("BIP0340/challenge".as_bytes());
    let mut engine = sha256::Hash::engine();
    engine.input(&tag.as_inner()[..]);
    engine.input(&tag.as_inner()[..]);
    engine
}
