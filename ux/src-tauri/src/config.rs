// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::tor::TorClient;
use crate::{tor::TorConfig, DBSelector, Database};
use event_log::connection::EventLog;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub db: Option<DBSelector>,
    pub tor: TorConfig,
    pub event_log: EventLogConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EventLogConfig {
    app_name: String,
    #[serde(default)]
    prefix: Option<PathBuf>,
}
impl Config {
    pub async fn get_evlog(self) -> Result<EventLog, Box<dyn Error+Send+Sync>> {
        let proj = format!("sapio-litigator.{}", self.event_log.app_name);
        let evlog = event_log::setup_db(&proj, self.event_log.prefix.clone())
            .await
            .map_err(|e| e.to_string())?;
        Ok(evlog)
    }
    pub async fn connect_to_db_if_set(&self, d: Database) -> Result<(), Box<dyn Error>> {
        if let Some(db) = &self.db {
            d.connect(&db.appname, db.prefix.clone()).await
        } else {
            Ok(())
        }
    }
}

pub struct Globals {
    pub config: Config,
    pub client: tokio::sync::OnceCell<TorClient>,
    pub evlog: tokio::sync::OnceCell<EventLog>,
}
impl Globals {
    pub fn new(config: Config) -> Arc<Self> {
        Arc::new(Self {
            evlog: Default::default(),
            client: Default::default(),
            config,
        })
    }
    pub async fn get_evlog(self: &Arc<Self>) -> Result<EventLog, Box<dyn Error+Send+Sync>> {
        let config = self.config.clone();
        self.evlog.get_or_try_init(move || config.get_evlog()).await.cloned()
    }
}
