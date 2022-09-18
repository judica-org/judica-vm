use rusqlite::named_params;

use crate::db_handle::{
    accessor_type::Get,
    accessors::occurrence_group::{OccurrenceGroup, OccurrenceGroupID, OccurrenceGroupKey},
    EventLogAccessor,
};

use super::{
    SQL_GET_OCCURRENCE_GROUPS, SQL_GET_OCCURRENCE_GROUP_BY_ID, SQL_GET_OCCURRENCE_GROUP_BY_KEY,
};

impl<'a, T> EventLogAccessor<'a, T>
where
    T: Get<OccurrenceGroup>,
{
    pub fn get_all_occurrence_groups(
        &self,
    ) -> Result<Vec<(OccurrenceGroupID, OccurrenceGroupKey)>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_OCCURRENCE_GROUPS)?;
        let q = stmt.query([])?;
        q.mapped(|row| {
            let r1 = row.get(0)?;
            let r2 = row.get(1)?;
            Ok((r1, r2))
        })
        .collect()
    }

    pub fn get_occurrence_group_by_id(
        &self,
        id: OccurrenceGroupID,
    ) -> Result<OccurrenceGroupKey, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_OCCURRENCE_GROUP_BY_ID)?;
        let q = stmt.query_row(named_params! {":group_id": id}, |row| {
            let r1: OccurrenceGroupKey = row.get(0)?;
            Ok(r1)
        })?;
        Ok(q)
    }

    pub fn get_occurrence_group_by_key(
        &self,
        key: OccurrenceGroupKey,
    ) -> Result<OccurrenceGroupID, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_OCCURRENCE_GROUP_BY_KEY)?;
        let q = stmt.query_row(named_params! {":group_key": key}, |row| {
            let r1: OccurrenceGroupID = row.get(0)?;
            Ok(r1)
        })?;
        Ok(q)
    }
}
