// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::db_handle::{
    handle_type::{Get, Insert},
    EventLogAccessor,
};

use super::Occurrence;

impl<'a, T> EventLogAccessor<'a, T> where T: Get<Occurrence> + Insert<Occurrence> {


}
