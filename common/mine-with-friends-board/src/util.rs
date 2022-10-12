// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::tokens::TokenPointer;

/// Convenience type to wrap a u128
pub(crate) type Price = u128;
/// More convenient name in some contexts
pub type Currency = TokenPointer;

pub type Watts = u128;
pub type Location = (i64, i64);
pub type ForSale = bool;
pub type HasMiners = bool;
