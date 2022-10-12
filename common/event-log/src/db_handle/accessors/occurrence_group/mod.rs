// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    db_handle::{DataType, Setup, SetupTrait},
    row_type_id,
};

use self::sql::{tables::SQL_OCCURRENCE_GROUP_TABLES, SQL_OCCURRENCE_CACHED_QUERIES};

pub mod sql;

pub struct OccurrenceGroup {}
impl DataType for OccurrenceGroup {}
impl SetupTrait for Setup<OccurrenceGroup> {
    fn setup_tables(&self) -> &'static str {
        SQL_OCCURRENCE_GROUP_TABLES
    }
    fn methods(&self) -> &'static [&'static [&'static str]] {
        SQL_OCCURRENCE_CACHED_QUERIES
    }
}

row_type_id!(OccurrenceGroupID);
pub type OccurrenceGroupKey = String;
