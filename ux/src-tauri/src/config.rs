use crate::tor::TorClient;
use crate::{tor::TorConfig, DBSelector, Database};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub db: Option<DBSelector>,
    pub tor: TorConfig,
}
impl Config {
    pub async fn connect_to_db_if_set(&self, d: Database) -> Result<(), Box<dyn Error>> {
        if let Some(db) = &self.db {
            d.connect(&db.appname, db.prefix.clone()).await
        } else {
            Ok(())
        }
    }
}

pub struct Globals {
    pub config: Config,
    pub client: tokio::sync::OnceCell<TorClient>,
}
impl Globals {
    pub fn new(config: Config) -> Arc<Self> {
        Arc::new(Self {
            config,
            client: Default::default(),
        })
    }
}
