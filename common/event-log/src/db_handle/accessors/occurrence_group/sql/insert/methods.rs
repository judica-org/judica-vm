// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use rusqlite::named_params;

use crate::db_handle::{
    accessor_type::Insert,
    accessors::occurrence_group::{OccurrenceGroup, OccurrenceGroupID, OccurrenceGroupKey},
    EventLogAccessor,
};

use super::SQL_NEW_OCCURRENCE_GROUP;

impl<'a, T> EventLogAccessor<'a, T>
where
    T: Insert<OccurrenceGroup>,
{
    pub fn insert_new_occurrence_group(
        &self,
        key: &OccurrenceGroupKey,
    ) -> Result<OccurrenceGroupID, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_NEW_OCCURRENCE_GROUP)?;
        let q = stmt.insert(named_params! {
            ":key": key,
        })?;
        Ok(OccurrenceGroupID(q))
    }
}
