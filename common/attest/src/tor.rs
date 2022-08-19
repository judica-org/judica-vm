use std::{any::Any, error::Error, fmt::Display, path::PathBuf, sync::Arc};

use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
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

pub fn start(
    config: Arc<Config>,
) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    tokio::task::spawn_blocking(move || {
        let mut buf = config.tor.directory.clone();
        buf.push("onion");
        let mut tor = Tor::new();
        tor.flag(TorFlag::DataDirectory(buf.to_str().unwrap().into()));

        buf.push("chatserver");
        let errc = match tor
            .flag(TorFlag::SocksPort(config.tor.socks_port))
            .flag(TorFlag::HiddenServiceDir(buf.to_str().unwrap().into()))
            .flag(TorFlag::HiddenServiceVersion(HiddenServiceVersion::V3))
            .flag(TorFlag::HiddenServicePort(
                TorAddress::Port(config.tor.attestation_port),
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
