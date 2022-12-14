// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use attest_database::connection::MsgDB;
use attest_database::setup_db;
use attest_util::bitcoin::BitcoinConfig;
use bitcoin::Network;
use bitcoincore_rpc_async::Client;
use event_log::connection::EventLog;

use serde::Deserialize;

use std::path::PathBuf;
use std::sync::Arc;

#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) db_app_name: String,
    #[serde(default)]
    pub(crate) db_prefix: Option<PathBuf>,
    pub(crate) bitcoin: BitcoinConfig,
    pub(crate) bitcoin_network: Network,
    pub(crate) app_instance: String,
    pub(crate) event_log: EventLogConfig,
}

#[derive(Deserialize)]
pub(crate) struct EventLogConfig {
    pub(crate) app_name: String,
    #[serde(default)]
    pub(crate) prefix: Option<PathBuf>,
}

impl Config {
    pub(crate) fn from_env() -> Result<Config, Box<dyn std::error::Error>> {
        let j = std::env::var("LITIGATOR_CONFIG_JSON")?;
        Ok(serde_json::from_str(&j)?)
    }
    pub(crate) async fn get_db(&self) -> Result<MsgDB, Box<dyn std::error::Error>> {
        let application = format!("attestations.{}", self.db_app_name);
        let db = setup_db(&application, self.db_prefix.clone()).await?;
        Ok(db)
    }
    pub(crate) async fn get_event_log(&self) -> Result<EventLog, Box<dyn std::error::Error>> {
        let proj = format!("sapio-litigator.{}", self.event_log.app_name);
        let db = event_log::setup_db(&proj, self.event_log.prefix.clone()).await?;
        Ok(db)
    }
    pub(crate) async fn get_bitcoin_rpc(&self) -> Result<Arc<Client>, Box<dyn std::error::Error>> {
        Ok(self.bitcoin.get_new_client().await?)
    }
}

pub(crate) fn data_dir_modules(app_instance: &str) -> PathBuf {
    let typ = "org";
    let org = "judica";
    let proj = format!("sapio-litigator.{}", app_instance);
    let proj =
        directories::ProjectDirs::from(typ, org, &proj).expect("Failed to find config directory");
    let mut data_dir = proj.data_dir().to_owned();
    data_dir.push("modules");
    data_dir
}
