use crate::{configuration::TorConfig, globals::Globals};
use attest_util::{ensure_dir, get_hidden_service_hostname, CrossPlatformPermissions, INFER_UNIT};
use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use std::{error::Error, fmt::Display, path::PathBuf, sync::Arc};
use tokio::{spawn, sync::Notify, task::JoinHandle};

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
        p.push("chatserver");
        Ok(
            ensure_dir(p, CrossPlatformPermissions::unix_only_permissions(0o700))
                .await
                .map_err(|e| format!("{}", e))?,
        )
    }
    pub async fn get_hostname(&self) -> Result<(String, u16), Box<dyn Error + Send + Sync>> {
        let hidden_service_dir = self.hidden_service_dir().await?;
        let s = get_hidden_service_hostname(hidden_service_dir).await?;
        Ok((s, self.exposed_application_port))
    }
}

pub async fn start(
    g: Arc<Globals>,
) -> Result<JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>, Box<dyn Error + Send + Sync>> {
    if let Some(tor_config) = g.config.tor.clone() {
        let data_dir = tor_config.root_dir().await?;
        let hidden_service_dir = tor_config.hidden_service_dir().await?;
        Ok(tokio::task::spawn_blocking(move || {
            let mut tor = Tor::new();
            tor.flag(TorFlag::DataDirectory(data_dir.to_str().unwrap().into()));

            let errc = match tor
                .flag(TorFlag::SocksPort(tor_config.socks_port))
                .flag(TorFlag::HiddenServiceDir(
                    hidden_service_dir.to_str().unwrap().into(),
                ))
                .flag(TorFlag::HiddenServiceVersion(HiddenServiceVersion::V3))
                .flag(TorFlag::HiddenServicePort(
                    TorAddress::Port(tor_config.exposed_application_port),
                    Some(TorAddress::Port(g.config.attestation_port)).into(),
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
