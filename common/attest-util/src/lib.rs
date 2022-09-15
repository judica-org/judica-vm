use std::{sync::Once, time::Instant};

static START: Once = Once::new();

static mut TIME: Option<Instant> = None;
static mut OFFSET: i64 = 0;

/// get the current time in milliseconds from UNIX_EPOCH
pub fn now() -> i64 {
    START.call_once(|| {
        let t2 = Instant::now();
        let t = std::time::SystemTime::now();
        let delta = t2.elapsed();
        let v = (t.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()
            - (delta.as_millis() / 2)) as i64;

        unsafe {
            OFFSET = v;
            TIME = Some(t2);
        }
    });
    let t = unsafe { OFFSET };
    let i = unsafe { TIME }.unwrap();
    i.elapsed().as_millis() as i64 + t
}

/// Helps with type inference
pub const INFER_UNIT: Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> = Ok(());
pub type AbstractResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

use std::fs::Permissions;
#[cfg(feature = "tokio")]
use std::{error::Error, path::PathBuf};
#[cfg(feature = "tokio")]
pub async fn ensure_dir(
    data_dir: PathBuf,
    perms: Option<Permissions>,
) -> Result<PathBuf, Box<dyn Error>> {
    let dir = tokio::fs::create_dir_all(&data_dir).await;
    match dir.as_ref().map_err(std::io::Error::kind) {
        Err(std::io::ErrorKind::AlreadyExists) => (),
        _e => dir?,
    };
    if let Some(perms) = perms {
        let _metadata = tokio::fs::set_permissions(&data_dir, perms).await?;
    }
    Ok(data_dir)
}

#[cfg(feature = "tokio")]
#[cfg(feature = "tracing")]
pub async fn get_hidden_service_hostname(
    mut hidden_service_dir: PathBuf,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    use std::time::Duration;
    use tracing::debug;
    use tracing::info;
    hidden_service_dir.push("hostname");
    loop {
        info!(location=?hidden_service_dir, "Checking for .onion Hostname");
        match tokio::fs::read_to_string(&hidden_service_dir).await {
            Ok(s) => {
                let s = s.trim();
                if s.ends_with(".onion") {
                    return Ok(s.into());
                } else {
                    debug!(?s, "Name not yet set");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
            Err(_) => {
                debug!("Name not yet set");
                tokio::time::sleep(Duration::from_secs(1)).await
            }
        }
    }
}

#[cfg(feature = "bitcoin")]
pub mod bitcoin {
    use bitcoincore_rpc_async as rpc;
    use rpc::Client;
    use serde::{Deserialize, Serialize};
    use std::{path::PathBuf, sync::Arc};
    /// The different authentication methods for the client.
    #[derive(Serialize, Deserialize)]
    #[serde(remote = "rpc::Auth")]
    pub enum Auth {
        None,
        UserPass(String, String),
        CookieFile(PathBuf),
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct BitcoinConfig {
        pub url: String,
        #[serde(with = "Auth")]
        pub auth: rpc::Auth,
    }
    impl BitcoinConfig {
        pub async fn get_new_client(&self) -> rpc::Result<Arc<Client>> {
            Ok(Arc::new(
                Client::new(self.url.clone(), self.auth.clone()).await?,
            ))
        }
    }
}
