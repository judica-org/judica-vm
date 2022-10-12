// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::db_handle::EventLogAccessor;
use crate::db_handle::accessor_type;
use rusqlite::Connection;
use std::{marker::PhantomData, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct EventLog(Arc<Mutex<Connection>>);

impl EventLog {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        EventLog(db)
    }
    pub async fn get_accessor(&self) -> EventLogAccessor<'static, accessor_type::All> {
        EventLogAccessor(self.0.clone().lock_owned().await, PhantomData::default())
    }
}
