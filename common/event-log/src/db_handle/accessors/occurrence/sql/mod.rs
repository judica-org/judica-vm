// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub use get::*;
pub use insert::*;
pub use tables::*;
pub use update::*;
pub mod insert;

pub mod update {}

pub mod get;
pub mod tables;

pub const SQL_OCCURRENCE_CACHED_QUERIES: &[&[&str]] = &[get::MANIFEST, insert::MANIFEST];
