use super::super::handle_type;
use super::super::ChainCommitGroupID;
use super::super::MessageID;
use super::super::MsgDBHandle;
use attest_messages::Authenticated;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::Envelope;
use rusqlite::named_params;

impl<'a, T> MsgDBHandle<'a, T>
where
    T: handle_type::Get,
{
    pub fn get_all_chain_commit_groups(
        &self,
    ) -> Result<Vec<(ChainCommitGroupID, String)>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(include_str!("../sql/get/all_chain_commit_groups.sql"))?;
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
            "../sql/get/all_chain_commit_groups_for_chain.sql"
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
            "../sql/get/all_chain_commit_group_members_for_chain.sql"
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
            "../sql/get/all_chain_commit_group_members_tips_for_chain.sql"
        ))?;
        let q = stmt.query(named_params! {":genesis_hash": genesis_hash})?;
        q.mapped(|row| {
            let r1 = row.get(0)?;
            Ok(r1)
        })
        .collect()
    }
}
