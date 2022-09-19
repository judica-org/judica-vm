use std::collections::HashMap;

use super::super::handle_type;
use super::super::ChainCommitGroupID;
use super::super::MessageID;
use super::super::MsgDBHandle;
use crate::db_handle::sql::get::chain_commit_groups::*;
use crate::sql_serializers::PK;
use attest_messages::AttestEnvelopable;
use attest_messages::Authenticated;
use attest_messages::CanonicalEnvelopeHash;

use attest_messages::GenericEnvelope;
use fallible_iterator::FallibleIterator;
use rusqlite::named_params;
use rusqlite::types::FromSql;
use sapio_bitcoin::XOnlyPublicKey;
use tracing::warn;

impl<'a, T> MsgDBHandle<'a, T>
where
    T: handle_type::Get,
{
    pub fn get_all_chain_commit_groups(
        &self,
    ) -> Result<Vec<(ChainCommitGroupID, String)>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_ALL_CHAIN_COMMIT_GROUPS)?;
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
        let mut stmt = self
            .0
            .prepare_cached(SQL_GET_ALL_CHAIN_COMMIT_GROUPS_FOR_CHAIN)?;
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
        let mut stmt = self
            .0
            .prepare_cached(SQL_GET_ALL_CHAIN_COMMIT_GROUP_MEMBERS_FOR_CHAIN)?;
        let q = stmt.query(named_params! {":genesis_hash": genesis_hash})?;
        q.mapped(|row| {
            let r1 = row.get(0)?;
            Ok(r1)
        })
        .collect()
    }

    pub fn get_all_chain_commit_group_members_tips_for_chain<M>(
        &self,
        key: XOnlyPublicKey,
        no_invalid_rows: bool,
    ) -> Result<Vec<Authenticated<GenericEnvelope<M>>>, rusqlite::Error>
    where
        M: AttestEnvelopable,
    {
        let mut stmt = self
            .0
            .prepare_cached(SQL_GET_ALL_CHAIN_COMMIT_GROUP_MEMBERS_TIPS_FOR_CHAIN)?;
        let mut q = stmt.query(named_params! {":key": PK(key)})?;
        let mut v: Vec<Authenticated<GenericEnvelope<M>>> = vec![];
        loop {
            match q.next() {
                Ok(o) => match o {
                    Some(row) => match row.get(0) {
                        Ok(r1) => {
                            v.push(r1);
                        }
                        Err(error) => {
                            warn!(?error, "Corrupt Row");
                            if no_invalid_rows {
                                return Err(error);
                            }
                        }
                    },
                    None => break,
                },
                Err(e) => return Err(e),
            }
        }
        Ok(v)
    }

    // Since messages uses autoincrement, 0 to start (min autoinc in 1)
    pub fn get_all_chain_commit_group_members_new_envelopes_for_chain_into_inconsistent<E, M>(
        &self,
        key: XOnlyPublicKey,
        newer: &mut i64,
        map: &mut HashMap<CanonicalEnvelopeHash, E>,
    ) -> Result<(), rusqlite::Error>
    where
        E: FromSql + AsRef<GenericEnvelope<M>>,
        M: AttestEnvelopable,
    {
        let mut stmt = self
            .0
            .prepare_cached(SQL_GET_ALL_CHAIN_COMMIT_GROUP_MEMBERS_NEW_ENVELOPES_FOR_CHAIN)?;
        let rows = stmt.query(named_params! {":key": PK(key), ":after": *newer})?;

        rows.map(|r| Ok((r.get::<_, E>(0)?, r.get::<_, i64>(1)?)))
            .for_each(|(v, id)| {
                map.insert(v.as_ref().canonicalized_hash_ref(), v);
                *newer = (*newer).max(id);
                Ok(())
            })?;
        Ok(())
    }
}
