use super::super::handle_type;
use super::super::MsgDBHandle;
use crate::db_handle::sql::get::messages::*;
use crate::db_handle::MessageID;
use attest_messages::AttestEnvelopable;
use attest_messages::Authenticated;
use attest_messages::CanonicalEnvelopeHash;

use attest_messages::GenericEnvelope;
use fallible_iterator::FallibleIterator;
use rusqlite::named_params;
use rusqlite::params;
use rusqlite::types::FromSql;
use rusqlite::OptionalExtension;
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
    pub fn get_connected_messages_newer_than_envelopes<'v, It, M>(
        &self,
        envelopes: It,
    ) -> Result<Vec<Authenticated<GenericEnvelope<M>>>, rusqlite::Error>
    where
        It: Iterator<Item = &'v Authenticated<GenericEnvelope<M>>>,
        M: AttestEnvelopable + 'v,
    {
        let mut stmt = self
            .0
            .prepare_cached(SQL_GET_MESSAGES_NEWER_THAN_FOR_GENESIS)?;
        let mut res = vec![];
        for envelope in envelopes {
            let rs = stmt
                .query(named_params! (":genesis": envelope.get_genesis_hash(), ":height": envelope.header().height()))?;
            let mut envs = rs
                .map(|r| r.get::<_, Authenticated<GenericEnvelope<M>>>(0))
                .collect()?;
            res.append(&mut envs);
        }
        Ok(res)
    }
    /// Returns the message at a given height for a key
    pub fn get_message_at_height_for_user<M>(
        &self,
        key: XOnlyPublicKey,
        height: u64,
    ) -> Result<Option<Authenticated<GenericEnvelope<M>>>, rusqlite::Error>
    where
        M: AttestEnvelopable,
    {
        let mut stmt = self.0.prepare_cached(SQL_GET_MESSAGES_BY_HEIGHT_AND_USER)?;
        stmt.query_row(params![key.to_hex(), height], |r| r.get(0))
            .optional()
    }

    /// finds the most recent message for a user by their key
    pub fn get_tip_for_user_by_key<M>(
        &self,
        key: XOnlyPublicKey,
    ) -> Result<Authenticated<GenericEnvelope<M>>, rusqlite::Error>
    where
        M: AttestEnvelopable,
    {
        let mut stmt = self.0.prepare_cached(SQL_GET_MESSAGES_TIPS_BY_USER)?;
        stmt.query_row([key.to_hex()], |r| r.get(0))
    }

    /// finds the most recent message only for messages where we know the key
    pub fn get_tip_for_known_keys<M>(
        &self,
    ) -> Result<Vec<Authenticated<GenericEnvelope<M>>>, rusqlite::Error>
    where
        M: AttestEnvelopable,
    {
        let mut stmt = self.0.prepare_cached(SQL_GET_TIPS_FOR_KNOWN_KEYS)?;
        let rows = stmt.query([])?;
        let vs: Vec<Authenticated<GenericEnvelope<M>>> = rows
            .map(|r| r.get::<_, Authenticated<GenericEnvelope<M>>>(0))
            .collect()?;
        Ok(vs)
    }

    pub fn get_disconnected_tip_for_known_keys<M>(
        &self,
    ) -> Result<Vec<Authenticated<GenericEnvelope<M>>>, rusqlite::Error>
    where
        M: AttestEnvelopable,
    {
        let mut stmt = self
            .0
            .prepare_cached(SQL_GET_DISCONNECTED_TIPS_FOR_KNOWN_KEYS)?;
        let rows = stmt.query([])?;
        let vs: Vec<Authenticated<GenericEnvelope<M>>> = rows
            .map(|r| r.get::<_, Authenticated<GenericEnvelope<M>>>(0))
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
    pub fn get_all_connected_messages_collect_into<M>(
        &self,
        newer: &mut Option<i64>,
        map: &mut HashMap<CanonicalEnvelopeHash, Authenticated<GenericEnvelope<M>>>,
    ) -> Result<(), rusqlite::Error>
    where
        M: AttestEnvelopable,
    {
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
        rows.map(|r| {
            Ok((
                r.get::<_, Authenticated<GenericEnvelope<M>>>(0)?,
                r.get::<_, i64>(1)?,
            ))
        })
        .for_each(|(v, id)| {
            map.insert(v.canonicalized_hash_ref(), v);
            *newer = (*newer).max(Some(id));
            Ok(())
        })?;
        Ok(())
    }

    pub fn get_all_messages_collect_into_inconsistent_skip_invalid<E, M>(
        &self,
        newer: &mut Option<i64>,
        map: &mut HashMap<CanonicalEnvelopeHash, E>,
        skip_invalid: bool,
    ) -> Result<(), rusqlite::Error>
    where
        E: FromSql + AsRef<GenericEnvelope<M>>,
        M: AttestEnvelopable,
    {
        let mut stmt = if newer.is_some() {
            self.0
                .prepare_cached(SQL_GET_ALL_MESSAGES_AFTER_INCONSISTENT)?
        } else {
            self.0.prepare_cached(SQL_GET_ALL_MESSAGES_INCONSISTENT)?
        };
        let mut rows = match newer {
            Some(i) => stmt.query([*i])?,
            None => stmt.query([])?,
        };
        while let Some(row) = rows.next()? {
            let id = row.get::<_, i64>(1)?;
            *newer = (*newer).max(Some(id));
            // skip invalid...
            match row.get::<_, E>(0) {
                Ok(v) => {
                    map.insert(v.as_ref().canonicalized_hash_ref(), v);
                }
                Err(e) if !skip_invalid => {
                    return Err(e);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// finds all most recent messages for all users
    pub fn get_tips_for_all_users<E, M>(&self) -> Result<Vec<E>, rusqlite::Error>
    where
        E: AsRef<GenericEnvelope<M>> + FromSql + std::fmt::Debug,
        M: AttestEnvelopable,
    {
        let mut stmt = self.0.prepare_cached(SQL_GET_ALL_TIPS_FOR_ALL_USERS)?;
        let rows = stmt.query([])?;
        let vs: Vec<E> = rows.map(|r| r.get::<_, E>(0)).collect()?;
        debug!(tips=?vs.iter().map(|e| (e.as_ref().header().height(), e.as_ref().get_genesis_hash())).collect::<Vec<_>>(), "Latest Tips Returned");
        trace!(envelopes=?vs, "Tips Returned");
        Ok(vs)
    }

    pub fn get_all_genesis<M>(
        &self,
    ) -> Result<Vec<Authenticated<GenericEnvelope<M>>>, rusqlite::Error>
    where
        M: AttestEnvelopable,
    {
        let mut stmt = self.0.prepare_cached(SQL_GET_ALL_GENESIS)?;
        let rows = stmt.query([])?;
        let vs: Vec<Authenticated<GenericEnvelope<M>>> = rows
            .map(|r| r.get::<_, Authenticated<GenericEnvelope<M>>>(0))
            .collect()?;
        debug!(tips=?vs.iter().map(|e| (e.header().height(), e.get_genesis_hash())).collect::<Vec<_>>(), "Genesis Tips Returned");
        trace!(envelopes=?vs, "Genesis Tips Returned");
        Ok(vs)
    }

    /// loads all the messages from a given user
    pub fn load_all_messages_for_user_by_key_connected<M>(
        &self,
        key: &sapio_bitcoin::secp256k1::XOnlyPublicKey,
    ) -> Result<Vec<Authenticated<GenericEnvelope<M>>>, rusqlite::Error>
    where
        M: AttestEnvelopable,
    {
        let mut stmt = self
            .0
            .prepare_cached(SQL_GET_ALL_MESSAGES_BY_KEY_CONNECTED)?;
        let rows = stmt.query(params![key.to_hex()])?;
        let vs: Vec<Authenticated<GenericEnvelope<M>>> = rows.map(|r| r.get(0)).collect()?;
        Ok(vs)
    }

    pub fn messages_by_hash<'i, I, E, M>(&self, hashes: I) -> Result<Vec<E>, rusqlite::Error>
    where
        I: Iterator<Item = &'i CanonicalEnvelopeHash>,
        E: AsRef<GenericEnvelope<M>> + FromSql,
        M: AttestEnvelopable,
    {
        let mut stmt = self.0.prepare_cached(SQL_GET_MESSAGE_BY_HASH)?;
        let r: Result<Vec<_>, _> = hashes
            .map(|hash| stmt.query_row([hash], |r| r.get::<_, E>(0)))
            .collect();
        r
    }
    pub fn messages_by_ids<'i, I, E, M>(&self, ids: I) -> Result<Vec<E>, rusqlite::Error>
    where
        I: Iterator<Item = &'i MessageID>,
        E: AsRef<GenericEnvelope<M>> + FromSql,
        M: AttestEnvelopable,
    {
        let mut stmt = self.0.prepare_cached(SQL_GET_MESSAGE_BY_ID)?;
        let r: Result<Vec<_>, _> = ids
            .map(|id| stmt.query_row([id], |r| r.get::<_, E>(0)))
            .collect();
        r
    }

    pub fn messages_by_id<E, M>(&self, id: MessageID) -> Result<E, rusqlite::Error>
    where
        E: AsRef<GenericEnvelope<M>> + FromSql,
        M: AttestEnvelopable,
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
