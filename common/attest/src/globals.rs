use crate::{attestations::client::AttestationClient, configuration::Config};
use sapio_bitcoin::secp256k1::{All, Secp256k1};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::OnceCell;
use tracing::info;

pub struct Globals {
    pub config: Arc<Config>,
    pub shutdown: AppShutdown,
    pub secp: Arc<Secp256k1<All>>,
    pub client: OnceCell<AttestationClient>,
}
impl Globals {
    pub async fn get_client(self: &Arc<Self>) -> Result<AttestationClient, reqwest::Error> {
        self.client
            .get_or_try_init(|| async {
                let mut bld = reqwest::Client::builder();
                if let Some(tor_config) = self.config.tor.clone() {
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
                }
                let inner_client = bld.build()?;
                let client = AttestationClient::new(inner_client);
                Ok(client)
            })
            .await
            .cloned()
    }
}

#[derive(Clone)]
pub struct AppShutdown {
    quit: Arc<AtomicBool>,
}

impl std::ops::Deref for AppShutdown {
    type Target = Arc<AtomicBool>;

    fn deref(&self) -> &Self::Target {
        &self.quit
    }
}

impl AppShutdown {
    pub fn new() -> Self {
        Self {
            quit: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn should_quit(&self) -> bool {
        self.quit.load(Ordering::Relaxed)
    }
    pub fn begin_shutdown(&self) {
        info!(event = "SHUTDOWN", "Beginning Node Shutdown",);
        self.quit.store(true, Ordering::Relaxed)
    }
}
