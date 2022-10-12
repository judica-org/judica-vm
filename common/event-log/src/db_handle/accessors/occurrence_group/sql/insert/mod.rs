// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod methods;
pub const SQL_NEW_OCCURRENCE_GROUP: &str = include_str!("new_occurrence_group.sql");
pub const MANIFEST: &[&str] = &[SQL_NEW_OCCURRENCE_GROUP];
