// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod methods;
pub use methods::Idempotent;
pub const SQL_NEW_OCCURRENCE: &str = include_str!("new_occurrence.sql");
pub const MANIFEST: &[&str] = &[SQL_NEW_OCCURRENCE];
