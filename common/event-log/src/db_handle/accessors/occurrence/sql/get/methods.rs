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

use super::{
    SQL_GET_OCCURRENCES_FOR_GROUP, SQL_GET_OCCURRENCES_FOR_GROUP_BY_TAG,
    SQL_GET_OCCURRENCE_AFTER_ID, SQL_GET_OCCURRENCE_BY_ID,
};

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
            let unique_tag: Option<String> = row.get(3)?;
            Ok(Occurrence {
                data: data.0,
                time,
                typeid,
                unique_tag,
            })
        })?;
        Ok(q)
    }
    pub fn get_occurrences_for_group(
        &self,
        id: OccurrenceGroupID,
    ) -> Result<Vec<(OccurrenceID, Occurrence)>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_OCCURRENCES_FOR_GROUP)?;
        let q = stmt.query_map(named_params! {":group_id": id}, |row| {
            let row_id: OccurrenceID = row.get(0)?;
            let data: SqlJson = row.get(1)?;
            let time: i64 = row.get(2)?;
            let typeid: ApplicationTypeID = row.get(3)?;
            let unique_tag: Option<String> = row.get(4)?;
            Ok((
                row_id,
                Occurrence {
                    data: data.0,
                    time,
                    typeid,
                    unique_tag,
                },
            ))
        })?;
        let v = q.collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    }

    pub fn get_occurrence_for_group_by_tag(
        &self,
        id: OccurrenceGroupID,
        tag: &str,
    ) -> Result<(OccurrenceID, Occurrence), rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(SQL_GET_OCCURRENCES_FOR_GROUP_BY_TAG)?;
        let q = stmt.query_row(named_params! {":group_id": id, ":tag": tag}, |row| {
            let row_id: OccurrenceID = row.get(0)?;
            let data: SqlJson = row.get(1)?;
            let time: i64 = row.get(2)?;
            let typeid: ApplicationTypeID = row.get(3)?;
            let unique_tag: Option<String> = row.get(4)?;
            Ok((
                row_id,
                Occurrence {
                    data: data.0,
                    time,
                    typeid,
                    unique_tag,
                },
            ))
        })?;
        Ok(q)
    }
    pub fn get_occurrences_for_group_after_id(
        &self,
        group_id: OccurrenceGroupID,
        after_id: OccurrenceID,
    ) -> Result<Vec<(OccurrenceID, Occurrence)>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_OCCURRENCE_AFTER_ID)?;
        let q = stmt.query_map(
            named_params! {":group_id": group_id, ":after_id": after_id},
            |row| {
                let row_id: OccurrenceID = row.get(0)?;
                let data: SqlJson = row.get(1)?;
                let time: i64 = row.get(2)?;
                let typeid: ApplicationTypeID = row.get(3)?;
                let unique_tag: Option<String> = row.get(4)?;
                Ok((
                    row_id,
                    Occurrence {
                        data: data.0,
                        time,
                        typeid,
                        unique_tag,
                    },
                ))
            },
        )?;
        let v = q.collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    }
}
