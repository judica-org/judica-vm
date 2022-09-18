use rusqlite::named_params;

use crate::{
    db_handle::{
        accessor_type::Insert,
        accessors::{
            occurrence::{Occurrence, OccurrenceID, ToOccurrence},
            occurrence_group::OccurrenceGroupID,
        },
        EventLogAccessor,
    },
    sql_serializers::SqlJsonRef,
};

use super::SQL_NEW_OCCURRENCE;

impl<'a, T> EventLogAccessor<'a, T>
where
    T: Insert<Occurrence>,
{
    pub fn insert_new_occurrence_now_from(
        &self,
        group_id: OccurrenceGroupID,
        data: &dyn ToOccurrence,
    ) -> Result<OccurrenceID, rusqlite::Error> {
        let occurrence = Occurrence::from(data);
        self.insert_occurrence(group_id, &occurrence)
    }

    pub fn insert_occurrence(
        &self,
        group_id: OccurrenceGroupID,
        occurrence: &Occurrence,
    ) -> Result<OccurrenceID, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_NEW_OCCURRENCE)?;
        let q = stmt.insert(named_params! {
            ":data": SqlJsonRef(&occurrence.data),
            ":time": occurrence.time,
            ":typeid": occurrence.typeid,
            ":group_id": group_id
        })?;
        Ok(OccurrenceID(q))
    }
}
