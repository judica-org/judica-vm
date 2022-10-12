// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod methods;
pub const SQL_GET_OCCURRENCES_FOR_GROUP: &str = include_str!("occurrence_for_group.sql");
pub const SQL_GET_OCCURRENCES_FOR_GROUP_BY_TAG: &str =
    include_str!("occurrence_for_group_by_tag.sql");
pub const SQL_GET_OCCURRENCE_AFTER_ID: &str = include_str!("occurrence_for_group_after_id.sql");
pub const SQL_GET_OCCURRENCE_BY_ID: &str = include_str!("occurrence_by_id.sql");
pub const MANIFEST: &[&str] = &[
    SQL_GET_OCCURRENCE_BY_ID,
    SQL_GET_OCCURRENCES_FOR_GROUP,
    SQL_GET_OCCURRENCES_FOR_GROUP_BY_TAG,
    SQL_GET_OCCURRENCE_AFTER_ID,
];
