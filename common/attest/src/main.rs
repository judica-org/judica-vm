// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use attest_database::connection::MsgDB;
use attest_util::INFER_UNIT;
use bitcoin_header_checkpoints::BitcoinCheckPointCache;
use globals::{AppShutdown, Globals};
use openssl_sys as _;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::mpsc::channel;
use tokio::task::JoinHandle;

use crate::attestations::server::protocol::GlobalSocketState;
mod attestations;
mod configuration;
mod control;
mod globals;
mod peer_services;
mod tor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().into_iter().collect();
    let config = match configuration::get_config() {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!("Trying to read config from file {}", e);
            if args.len() != 2 {
                Err("Expected only 2 args, file name of config")?;
            }
            let config: Arc<configuration::Config> = Arc::new(serde_json::from_slice(
                &tokio::fs::read(&args[1]).await?[..],
            )?);
            config
        }
    };
    tracing::debug!("Opening DB");
    let msg_db = config.setup_db().await?;
    tracing::debug!("Database Connection Setup");
    let g = Arc::new(Globals {
        config,
        shutdown: AppShutdown::new(),
        secp: Default::default(),
        client: Default::default(),
        msg_db,
        socket_state: GlobalSocketState::default(),
    });
    init_main(g).await
}
async fn init_main(g: Arc<Globals>) -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing::debug!("Config Loaded");
    let bitcoin_client = g.config.bitcoin.get_new_client().await?;
    tracing::debug!("Bitcoin Client Loaded");
    let bitcoin_checkpoints = Arc::new(
        BitcoinCheckPointCache::new(bitcoin_client, None, (*g.shutdown.clone()).clone()).await,
    );
    let mut checkpoint_service = bitcoin_checkpoints
        .run_cache_service()
        .ok_or("Checkpoint service already started")?;
    tracing::debug!("Checkpoint Service Started");
    let mut attestation_server = attestations::server::run(g.clone(), g.msg_db.clone()).await;
    let mut tor_service = tor::start(g.clone()).await?;
    let (tx_peer_status, rx_peer_status) = channel(1);
    let mut fetching_client = peer_services::startup(g.clone(), g.msg_db.clone(), rx_peer_status);
    let mut control_server = control::server::run(
        g.clone(),
        g.msg_db.clone(),
        tx_peer_status,
        bitcoin_checkpoints,
    )
    .await;

    tracing::debug!("Starting Subservices");
    let mut skip = None;
    tokio::select!(
    a = &mut attestation_server => {
        tracing::debug!("Error From Attestation Server: {:?}", a);
        skip.replace("attest");
    },
    b = &mut tor_service => {
        tracing::debug!("Error From Tor Server: {:?}", b);
        skip.replace("tor");
    },
    c = &mut fetching_client => {
        tracing::debug!("Error From Fetching Server: {:?}", c);
        skip.replace("fetch");
    },
    d = &mut checkpoint_service => {
        tracing::debug!("Error From Checkpoint Server: {:?}", d);
        skip.replace("checkpoint");
    }
    e = &mut control_server => {
        tracing::debug!("Error From Control Server: {:?}", e);
        skip.replace("control");
    });
    tracing::debug!("Shutting Down Subservices");
    g.shutdown.begin_shutdown();
    let svcs = [
        ("tor", tor_service),
        ("attest", attestation_server),
        ("fetch", fetching_client),
        ("checkpoint", checkpoint_service),
        ("control", control_server),
    ];
    for svc in &svcs {
        tracing::debug!("Abort Subservice: {}", svc.0);
        svc.1.abort();
    }
    futures::future::join_all(svcs.into_iter().filter_map(|x| {
        if Some(x.0) == skip {
            tracing::debug!("Skipping Wait for Terminated Subservice: {}", x.0);
            None
        } else {
            tracing::debug!("Waiting for Subservice: {}", x.0);
            Some(x.1)
        }
    }))
    .await;

    tracing::debug!("Exiting");
    INFER_UNIT
}

#[cfg(test)]
mod test;
