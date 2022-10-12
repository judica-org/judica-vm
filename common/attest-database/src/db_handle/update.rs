// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::handle_type;
use super::MsgDBHandle;
use crate::db_handle::sql::update::*;
impl<T> MsgDBHandle<T>
where
    T: handle_type::Get + handle_type::Insert,
{
    /// Normally not required, as triggered on DB insert
    pub fn resolve_parents(&mut self) -> Result<(), rusqlite::Error> {
        let mut s = self.0.prepare_cached(SQL_UPDATE_CONNECT_PARENTS)?;
        s.execute([])?;
        Ok(())
    }
    /// Required to run periodically to make progress...
    /// TODO: Something more efficient?
    pub fn attach_tips(&self) -> Result<usize, rusqlite::Error> {
        let mut s = self.0.prepare_cached(SQL_UPDATE_CONNECT_RECURSIVE)?;
        s.execute([])
    }

    /// adds a hidden service to our connection list
    /// Won't fail if already exists
    pub fn upsert_hidden_service(
        &self,
        s: String,
        port: u16,
        fetch_from: Option<bool>,
        push_to: Option<bool>,
        allow_unsolicited_tips: Option<bool>,
    ) -> Result<(), rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_UPDATE_HIDDEN_SERVICE)?;
        stmt.insert(rusqlite::named_params!(
            ":service_url": s,
            ":port": port,
            ":fetch_from": fetch_from,
            ":push_to": push_to,
            ":allow_unsolicited_tips": allow_unsolicited_tips
        ))?;
        Ok(())
    }
}
