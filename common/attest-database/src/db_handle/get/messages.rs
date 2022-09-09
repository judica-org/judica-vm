use super::super::handle_type;
use super::super::MsgDBHandle;
use crate::db_handle::sql::get::messages::*;
use crate::db_handle::MessageID;
use attest_messages::Authenticated;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::Envelope;
use fallible_iterator::FallibleIterator;
use rusqlite::named_params;
use rusqlite::params;
use rusqlite::types::FromSql;
use sapio_bitcoin;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::hashes::sha256;
use sapio_bitcoin::XOnlyPublicKey;
use std::collections::HashMap;
use tracing::debug;
use tracing::trace;

impl<'a, T> MsgDBHandle<'a, T>
where
    T: handle_type::Get,
{
    /// Returns the message at a given height for a key
    pub fn get_connected_messages_newer_than_envelopes<'v, It>(
        &self,
        envelopes: It,
    ) -> Result<Vec<Authenticated<Envelope>>, rusqlite::Error>
    where
        It: Iterator<Item = &'v Authenticated<Envelope>>,
    {
        let mut stmt = self
            .0
            .prepare_cached(SQL_GET_MESSAGES_NEWER_THAN_FOR_GENESIS)?;
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
        let mut stmt = self.0.prepare_cached(SQL_GET_MESSAGES_BY_HEIGHT_AND_USER)?;
        stmt.query_row(params![key.to_hex(), height], |r| r.get(0))
    }

    /// finds the most recent message for a user by their key
    pub fn get_tip_for_user_by_key(
        &self,
        key: XOnlyPublicKey,
    ) -> Result<Authenticated<Envelope>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_MESSAGES_TIPS_BY_USER)?;
        stmt.query_row([key.to_hex()], |r| r.get(0))
    }

    /// finds the most recent message only for messages where we know the key
    pub fn get_tip_for_known_keys(&self) -> Result<Vec<Authenticated<Envelope>>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_TIPS_FOR_KNOWN_KEYS)?;
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
            .prepare_cached(SQL_GET_DISCONNECTED_TIPS_FOR_KNOWN_KEYS)?;
        let rows = stmt.query([])?;
        let vs: Vec<Authenticated<Envelope>> = rows
            .map(|r| r.get::<_, Authenticated<Envelope>>(0))
            .collect()?;
        Ok(vs)
    }

    #[cfg(test)]
    pub fn drop_message_by_hash(&self, h: CanonicalEnvelopeHash) -> Result<(), rusqlite::Error> {
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
                .prepare_cached(SQL_GET_ALL_MESSAGES_AFTER_CONNECTED)?
        } else {
            self.0.prepare_cached(SQL_GET_ALL_MESSAGES_CONNECTED)?
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
                .prepare_cached(SQL_GET_ALL_MESSAGES_AFTER_INCONSISTENT)?
        } else {
            self.0.prepare_cached(SQL_GET_ALL_MESSAGES_INCONSISTENT)?
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

    /// finds all most recent messages for all users
    pub fn get_tips_for_all_users<E>(&self) -> Result<Vec<E>, rusqlite::Error>
    where
        E: AsRef<Envelope> + FromSql + std::fmt::Debug,
    {
        let mut stmt = self.0.prepare_cached(SQL_GET_ALL_TIPS_FOR_ALL_USERS)?;
        let rows = stmt.query([])?;
        let vs: Vec<E> = rows.map(|r| r.get::<_, E>(0)).collect()?;
        debug!(tips=?vs.iter().map(|e| (e.as_ref().header().height(), e.as_ref().get_genesis_hash())).collect::<Vec<_>>(), "Latest Tips Returned");
        trace!(envelopes=?vs, "Tips Returned");
        Ok(vs)
    }

    pub fn get_all_genesis(&self) -> Result<Vec<Authenticated<Envelope>>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_ALL_GENESIS)?;
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
            .prepare_cached(SQL_GET_ALL_MESSAGES_BY_KEY_CONNECTED)?;
        let rows = stmt.query(params![key.to_hex()])?;
        let vs: Vec<Authenticated<Envelope>> = rows.map(|r| r.get(0)).collect()?;
        Ok(vs)
    }

    pub fn messages_by_hash<'i, I, E>(&self, hashes: I) -> Result<Vec<E>, rusqlite::Error>
    where
        I: Iterator<Item = &'i CanonicalEnvelopeHash>,
        E: AsRef<Envelope> + FromSql,
    {
        let mut stmt = self.0.prepare_cached(SQL_GET_MESSAGE_BY_HASH)?;
        let r: Result<Vec<_>, _> = hashes
            .map(|hash| stmt.query_row([hash], |r| r.get::<_, E>(0)))
            .collect();
        r
    }

    pub fn messages_by_id<'i, E>(&self, id: MessageID) -> Result<E, rusqlite::Error>
    where
        E: AsRef<Envelope> + FromSql,
    {
        let mut stmt = self.0.prepare_cached(SQL_GET_MESSAGE_BY_ID)?;
        stmt.query_row([id], |r| r.get::<_, E>(0))
    }

    pub fn message_exists(&self, hash: &sha256::Hash) -> Result<bool, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_MESSAGE_EXISTS)?;
        stmt.exists([hash.to_hex()])
    }
    pub fn message_not_exists_it<'i, I>(
        &self,
        hashes: I,
    ) -> Result<Vec<CanonicalEnvelopeHash>, rusqlite::Error>
    where
        I: Iterator<Item = &'i CanonicalEnvelopeHash>,
    {
        let mut stmt = self.0.prepare_cached(SQL_GET_MESSAGE_EXISTS)?;
        hashes
            .filter_map(|hash| match stmt.exists([hash]) {
                Ok(true) => None,
                Ok(false) => Some(Ok(*hash)),
                Err(x) => Some(Err(x)),
            })
            .collect()
    }
}
