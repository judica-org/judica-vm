use std::{error::Error, fmt::Display, sync::Arc};

use attest_util::{ensure_dir, INFER_UNIT};
use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use tokio::{spawn, sync::Notify, task::JoinHandle};

use crate::configuration::{Config, self};

#[derive(Debug)]
pub enum TorError {
    Code(u8),
    Error(libtor::Error),
}

impl Display for TorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for TorError {}

pub async fn start(
    config: Arc<configuration::Config>,
) -> Result<JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>, Box<dyn Error + Send + Sync>> {
    if let Some(tor_config) = config.tor.clone() {
        ensure_dir(tor_config.directory.clone())
            .await
            .map_err(|e| format!("{}", e))?;
        Ok(tokio::task::spawn_blocking(move || {
            let mut buf = tor_config.directory.clone();
            buf.push("onion");
            let mut tor = Tor::new();
            tor.flag(TorFlag::DataDirectory(buf.to_str().unwrap().into()));

            buf.push("chatserver");
            let errc = match tor
                .flag(TorFlag::SocksPort(tor_config.socks_port))
                .flag(TorFlag::HiddenServiceDir(buf.to_str().unwrap().into()))
                .flag(TorFlag::HiddenServiceVersion(HiddenServiceVersion::V3))
                .flag(TorFlag::HiddenServicePort(
                    TorAddress::Port(config.attestation_port),
                    None.into(),
                ))
                .start_background()
                .join()
                .map_err(|_| "Join Error at Thread Level")?
            {
                Ok(u) => TorError::Code(u),
                Err(e) => TorError::Error(e),
            };
            Err(errc)?
        }))
    } else {
        Ok(spawn(async {
            let v = Notify::new();
            v.notified().await;
            INFER_UNIT
        }))
    }
}
