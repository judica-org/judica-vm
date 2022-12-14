// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Helps with type inference
pub const INFER_UNIT: Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> = Ok(());
pub type AbstractResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;
