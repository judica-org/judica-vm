use crate::config::Globals;
use attest_messages::{Authenticated, GenericEnvelope};
use attest_util::{ensure_dir, CrossPlatformPermissions};
use game_host_messages::{CreatedNewChain, FinishArgs, JoinCode, NewGame};
use game_player_messages::ParticipantAction;
use libtor::{HiddenServiceVersion, Tor, TorAddress, TorFlag};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{Debug, Display},
    path::PathBuf,
    sync::Arc,
};
use tokio::task::JoinHandle;

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

pub async fn start(globals: Arc<Globals>) -> JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    let root_dir = globals.config.tor.root_dir().await;
    tokio::task::spawn_blocking(move || {
        let mut tor = Tor::new();

        let errc = match tor
            .flag(TorFlag::DataDirectory(
                root_dir?.to_str().unwrap().to_owned(),
            ))
            .flag(TorFlag::SocksPort(globals.config.tor.socks_port))
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
#[derive(Clone, Serialize, Deserialize, Debug, JsonSchema)]
pub struct GameHost {
    pub url: String,
    pub port: u16,
}
const GAME_NEW: &str = "game/new";
const GAME_ADD_PLAYER: &str = "game/player/new";
const GAME_FINISH_SETUP: &str = "game/finish";

trait DebugErr<R, E> {
    fn debug_err(self) -> Result<R, E>;
}
impl<R, E> DebugErr<R, E> for Result<R, E>
where
    E: Debug,
{
    fn debug_err(self) -> Result<R, E> {
        if let Err(e) = self.as_ref() {
            tracing::debug!(error=?e, "Request Failed");
        }
        self
    }
}

impl TorClient {
    pub async fn create_new_game_instance(
        &self,
        GameHost { url, port }: &GameHost,
        minutes: u16,
    ) -> Result<NewGame, reqwest::Error> {
        self.client
            .post(format!("http://{}:{}/{}", url, port, GAME_NEW))
            .json(&NewGameArgs{duration_minutes:minutes})
            .send()
            .await
            .debug_err()?
            .json()
            .await
            .debug_err()
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
            .await
            .debug_err()?
            .json()
            .await
            .debug_err()
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
            .await
            .debug_err()?
            .json()
            .await
            .debug_err()
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
                let proxy =
                    reqwest::Proxy::all(format!("socks5h://127.0.0.1:{}", tor_config.socks_port))?;
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
