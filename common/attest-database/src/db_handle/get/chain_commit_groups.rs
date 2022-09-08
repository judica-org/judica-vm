use crate::db_handle::sql::get::chain_commit_groups::*;
use crate::sql_serializers::PK;
use super::super::handle_type;
use super::super::ChainCommitGroupID;
use super::super::MessageID;
use super::super::MsgDBHandle;
use attest_messages::Authenticated;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::Envelope;
use rusqlite::named_params;
use sapio_bitcoin::XOnlyPublicKey;

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

    pub fn get_all_chain_commit_group_members_tips_for_chain(
        &self,
        key: XOnlyPublicKey,
    ) -> Result<Vec<Authenticated<Envelope>>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(SQL_GET_ALL_CHAIN_COMMIT_GROUP_MEMBERS_TIPS_FOR_CHAIN)?;
        let q = stmt.query(named_params! {":key": PK(key)})?;
        q.mapped(|row| {
            let r1 = row.get(0)?;
            Ok(r1)
        })
        .collect()
    }
}
