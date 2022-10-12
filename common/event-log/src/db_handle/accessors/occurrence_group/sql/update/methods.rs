// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::db_handle::{
    accessor_type::{Get, Insert},
    accessors::occurrence_group::OccurrenceGroup,
    EventLogAccessor,
};

impl<'a, T> EventLogAccessor<'a, T> where T: Get<OccurrenceGroup> + Insert<OccurrenceGroup> {}
