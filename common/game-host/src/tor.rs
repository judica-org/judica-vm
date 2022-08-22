use std::{error::Error, fmt::Display, path::PathBuf, sync::Arc};

use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use serde::{Serialize, Deserialize};
use tokio::task::JoinHandle;

use crate::Config;

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

#[derive(Serialize, Deserialize)]
pub struct TorConfig {
    pub directory: PathBuf,
    pub socks_port: u16,
    pub application_port: u16,
    pub application_path: String,
}
pub fn start(config: Arc<Config>) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    tokio::task::spawn_blocking(move || {
        let mut buf = config.tor.directory.clone();
        buf.push("onion");
        let mut tor = Tor::new();
        tor.flag(TorFlag::DataDirectory(buf.to_str().unwrap().into()));

        buf.push(&config.tor.application_path);
        let errc = match tor
            .flag(TorFlag::SocksPort(config.tor.socks_port))
            .flag(TorFlag::HiddenServiceDir(buf.to_str().unwrap().into()))
            .flag(TorFlag::HiddenServiceVersion(HiddenServiceVersion::V3))
            .flag(TorFlag::HiddenServicePort(
                TorAddress::Port(config.tor.application_port),
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
    })
}
