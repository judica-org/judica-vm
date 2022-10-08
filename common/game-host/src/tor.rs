use attest_util::{ensure_dir, get_hidden_service_hostname, CrossPlatformPermissions};

use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt::Display, path::PathBuf, sync::Arc};
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
    pub exposed_application_port: u16,
    pub application_path: String,
}
impl TorConfig {
    async fn root_dir(&self) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        let mut buf = self.directory.clone();
        buf.push("onion");
        Ok(
            ensure_dir(buf, CrossPlatformPermissions::unix_only_permissions(0o700))
                .await
                .map_err(|e| format!("{}", e))?,
        )
    }
    async fn hidden_service_dir(&self) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        let mut p = self.root_dir().await?;
        p.push(&self.application_path);
        Ok(
            ensure_dir(p, CrossPlatformPermissions::unix_only_permissions(0o700))
                .await
                .map_err(|e| format!("{}", e))?,
        )
    }
    pub async fn get_hostname(&self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let hidden_service_dir = self.hidden_service_dir().await?;
        get_hidden_service_hostname(hidden_service_dir).await
    }
}

pub async fn start(config: Arc<Config>) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    let root_dir = config.tor.root_dir().await;
    let hidden_service_dir = config.tor.hidden_service_dir().await;
    tokio::task::spawn_blocking(move || {
        let mut tor = Tor::new();

        let errc = match tor
            .flag(TorFlag::DataDirectory(
                root_dir?.to_str().unwrap().to_owned(),
            ))
            .flag(TorFlag::SocksPort(config.tor.socks_port))
            .flag(TorFlag::HiddenServiceDir(
                hidden_service_dir?.to_str().unwrap().to_owned(),
            ))
            .flag(TorFlag::HiddenServiceVersion(HiddenServiceVersion::V3))
            .flag(TorFlag::HiddenServicePort(
                TorAddress::Port(config.tor.exposed_application_port),
                Some(TorAddress::Port(config.tor.application_port)).into(),
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
