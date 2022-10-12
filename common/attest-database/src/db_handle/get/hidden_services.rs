// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::PeerInfo;
use crate::db_handle::{handle_type, sql::SQL_GET_ALL_HIDDEN_SERVICES, MsgDBHandle};
use fallible_iterator::FallibleIterator;

impl<T> MsgDBHandle<T>
where
    T: handle_type::Get,
{
    /// get all added hidden services
    pub fn get_all_hidden_services(&self) -> Result<Vec<PeerInfo>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_ALL_HIDDEN_SERVICES)?;
        let results = stmt
            .query([])?
            .map(|r| {
                let service_url = r.get::<_, String>(0)?;
                let port = r.get(1)?;
                let fetch_from = r.get(2)?;
                let push_to = r.get(3)?;
                let allow_unsolicited_tips = r.get(4)?;
                Ok(PeerInfo {
                    service_url,
                    port,
                    fetch_from,
                    push_to,
                    allow_unsolicited_tips,
                })
            })
            .collect()?;
        Ok(results)
    }
}
