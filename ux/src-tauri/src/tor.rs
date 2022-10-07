use attest_messages::{GenericEnvelope, Authenticated};
use attest_util::{ensure_dir, CrossPlatformPermissions};
use game_host_messages::{JoinCode, NewGame, FinishArgs, CreatedNewChain};
use game_player_messages::ParticipantAction;
use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt::Display, path::PathBuf, sync::Arc};
use tokio::task::JoinHandle;
use crate::config::Globals;

use crate::{Config, Game};

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

#[derive(Serialize, Deserialize, Clone)]
pub struct TorConfig {
    pub directory: PathBuf,
    pub socks_port: u16,
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
}

pub async fn start(config: Arc<Config>) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    let root_dir = config.tor.root_dir().await;
    tokio::task::spawn_blocking(move || {
        let mut tor = Tor::new();

        let errc = match tor
            .flag(TorFlag::DataDirectory(
                root_dir?.to_str().unwrap().to_owned(),
            ))
            .flag(TorFlag::SocksPort(config.tor.socks_port))
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

#[derive(Clone)]
pub struct TorClient {
    client: reqwest::Client,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct GameHost {
    pub url: String,
    pub port: u16,
}
const GAME_NEW: &str = "game/new";
const GAME_ADD_PLAYER: &str = "game/player/new";
const GAME_FINISH_SETUP: &str = "game/finish";
impl TorClient {
    pub async fn create_new_game_instance(
        &self,
        GameHost { url, port }: &GameHost,
    ) -> Result<NewGame, reqwest::Error> {
        self.client
            .post(format!("http://{}:{}/{}", url, port, GAME_NEW))
            .send()
            .await?
            .json()
            .await
    }

    pub async fn add_player(
        &self,
        GameHost { url, port }: &GameHost,
        args: (JoinCode, Authenticated<GenericEnvelope<ParticipantAction>>),
    ) -> Result<(), reqwest::Error> {
        self.client
            .post(format!("http://{}:{}/{}", url, port, GAME_ADD_PLAYER))
            .json(&args)
            .send()
            .await?
            .json()
            .await
    }

    pub async fn finish_setup(
        &self,
        GameHost { url, port }: &GameHost,
        f: FinishArgs,
    ) -> Result<CreatedNewChain, reqwest::Error> {
        self.client
            .post(format!("http://{}:{}/{}", url, port, GAME_FINISH_SETUP))
            .json(&f)
            .send()
            .await?
            .json()
            .await
    }
}

impl Globals {
    pub async fn get_client(self: &Arc<Globals>) -> Result<TorClient, reqwest::Error> {
            self.client
                .get_or_try_init(|| async {
                let mut bld = reqwest::Client::builder();
            let tor_config = self.config.tor.clone();
                    // Local Pass if in test mode
                    // TODO: make this programmatic?
                    #[cfg(test)]
                    {
                        bld = bld.proxy(reqwest::Proxy::custom(move |url| {
                            if url.host_str() == Some("127.0.0.1") {
                                Some("127.0.0.1")
                            } else {
                                None
                            }
                        }));
                    }
                    let proxy = reqwest::Proxy::all(format!(
                        "socks5h://127.0.0.1:{}",
                        tor_config.socks_port
                    ))?;
                    bld = bld.proxy(proxy);
                let inner_client = bld.build()?;
                let client = TorClient {
                    client: inner_client,
                };
                Ok(client)
            })
            .await
            .cloned()
    }
}
