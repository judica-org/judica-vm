use rusqlite::named_params;

use crate::{
    db_handle::{
        accessor_type::Get,
        accessors::{
            occurrence::{ApplicationTypeID, Occurrence, OccurrenceID},
            occurrence_group::OccurrenceGroupID,
        },
        EventLogAccessor,
    },
    sql_serializers::SqlJson,
};

use super::{SQL_GET_OCCURRENCES_FOR_GROUP, SQL_GET_OCCURRENCE_AFTER_ID, SQL_GET_OCCURRENCE_BY_ID};

impl<'a, T> EventLogAccessor<'a, T>
where
    T: Get<Occurrence>,
{
    pub fn get_occurrence(&self, id: OccurrenceID) -> Result<Occurrence, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_OCCURRENCE_BY_ID)?;
        let q = stmt.query_row(named_params! {":id": id}, |row| {
            let data: SqlJson = row.get(0)?;
            let time: i64 = row.get(1)?;
            let typeid: ApplicationTypeID = row.get(2)?;
            Ok(Occurrence {
                data: data.0,
                time,
                typeid,
            })
        })?;
        Ok(q)
    }
    pub fn get_occurrences_for_group(
        &self,
        id: OccurrenceGroupID,
    ) -> Result<Vec<Occurrence>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_OCCURRENCES_FOR_GROUP)?;
        let q = stmt.query_map(named_params! {":group_id": id}, |row| {
            let data: SqlJson = row.get(0)?;
            let time: i64 = row.get(1)?;
            let typeid: ApplicationTypeID = row.get(2)?;
            Ok(Occurrence {
                data: data.0,
                time,
                typeid,
            })
        })?;
        let v = q.collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    }
    pub fn get_occurrences_for_group_after_id(
        &self,
        group_id: OccurrenceGroupID,
        after_id: OccurrenceID,
    ) -> Result<Vec<Occurrence>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_OCCURRENCE_AFTER_ID)?;
        let q = stmt.query_map(
            named_params! {":group_id": group_id, ":after_id": after_id},
            |row| {
                let data: SqlJson = row.get(0)?;
                let time: i64 = row.get(1)?;
                let typeid: ApplicationTypeID = row.get(2)?;
                Ok(Occurrence {
                    data: data.0,
                    time,
                    typeid,
                })
            },
        )?;
        let v = q.collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    }
}
