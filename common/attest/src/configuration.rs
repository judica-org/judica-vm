// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use attest_database::connection::MsgDB;
use attest_database::setup_db;
use attest_database::setup_test_db;
use attest_util::bitcoin::BitcoinConfig;

use sapio_bitcoin::secp256k1::rand;
use sapio_bitcoin::secp256k1::rand::Rng;
use serde::Deserialize;
use serde::Serialize;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::{Interval, MissedTickBehavior};

pub(crate) const fn default_port() -> u16 {
    46789
}

pub(crate) fn default_socks_port() -> u16 {
    19050
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TorConfig {
    pub(crate) directory: PathBuf,
    #[serde(default = "default_socks_port")]
    pub(crate) socks_port: u16,
    #[serde(default = "default_exposed_port")]
    pub(crate) exposed_application_port: u16,
}

pub(crate) fn default_control_port() -> u16 {
    14322
}

pub(crate) fn default_exposed_port() -> u16 {
    26874
}

#[derive(Serialize, Deserialize)]
pub struct ControlConfig {
    #[serde(default = "default_control_port")]
    pub(crate) port: u16,
}

#[derive(Serialize, Deserialize)]
pub struct PeerServicesTimers {
    pub reconnect_rate: Duration,
    pub scan_for_unsent_tips_rate: Duration,
    pub attach_tip_while_busy_rate: Duration,
    pub tip_fetch_rate: Duration,
    pub entropy_range: Duration,
}

impl PeerServicesTimers {
    pub(crate) fn scaled_default(scale: f64) -> Self {
        Self {
            reconnect_rate: Duration::from_millis((30000_f64 * scale) as u64),
            scan_for_unsent_tips_rate: Duration::from_millis((10000_f64 * scale) as u64),
            attach_tip_while_busy_rate: Duration::from_millis((30000_f64 * scale) as u64),
            tip_fetch_rate: Duration::from_millis((15000_f64 * scale) as u64),
            entropy_range: Duration::from_millis((1000_f64 * scale) as u64),
        }
    }
}

impl Default for PeerServicesTimers {
    fn default() -> Self {
        Self::scaled_default(1.0)
    }
}

impl PeerServicesTimers {
    pub(crate) fn rand(&self) -> Duration {
        rand::thread_rng().gen_range(Duration::ZERO, self.entropy_range)
    }
    pub(crate) fn reconnect_interval(&self) -> Interval {
        let mut interval = tokio::time::interval(self.reconnect_rate);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        interval
    }
    pub(crate) async fn scan_for_unsent_tips_delay(&self) {
        let d = self.scan_for_unsent_tips_rate + self.rand();
        tokio::time::sleep(d).await
    }
    pub(crate) async fn tip_fetch_delay(&self) {
        let d = self.tip_fetch_rate + self.rand();
        tokio::time::sleep(d).await
    }
    // todo: add randomization
    pub(crate) fn attach_tip_while_busy_interval(&self) -> Interval {
        let mut interval = tokio::time::interval(self.attach_tip_while_busy_rate);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        interval
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct PeerServiceConfig {
    #[serde(default)]
    pub timer_override: PeerServicesTimers,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub(crate) bitcoin: BitcoinConfig,
    pub subname: String,
    pub tor: Option<TorConfig>,
    #[serde(default = "default_port")]
    pub attestation_port: u16,
    pub control: ControlConfig,
    #[serde(default)]
    pub prefix: Option<PathBuf>,
    #[serde(default)]
    pub peer_service: PeerServiceConfig,
    #[serde(skip, default)]
    pub test_db: bool,
}

pub(crate) fn get_config() -> Result<Arc<Config>, Box<dyn Error>> {
    let config = std::env::var("ATTEST_CONFIG_JSON").map(|s| serde_json::from_str(&s))??;
    Ok(Arc::new(config))
}

impl Config {
    pub async fn setup_db(&self) -> Result<MsgDB, Box<dyn Error + Send + Sync>> {
        if self.test_db {
            Ok(setup_test_db().await)
        } else {
            let application = format!("attestations.{}", self.subname);
            let mdb = setup_db(&application, self.prefix.clone())
                .await
                .map_err(|e| format!("{}", e))?;
            Ok(mdb)
        }
    }
}
