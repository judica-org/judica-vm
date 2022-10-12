// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod methods;
pub const SQL_GET_OCCURRENCE_GROUPS: &str = include_str!("occurrence_groups.sql");
pub const SQL_GET_OCCURRENCE_GROUP_BY_KEY: &str = include_str!("occurrence_group_by_key.sql");
pub const SQL_GET_OCCURRENCE_GROUP_BY_ID: &str = include_str!("occurrence_group_by_id.sql");
pub const MANIFEST: &[&str] = &[
    SQL_GET_OCCURRENCE_GROUPS,
    SQL_GET_OCCURRENCE_GROUP_BY_KEY,
    SQL_GET_OCCURRENCE_GROUP_BY_ID,
];
