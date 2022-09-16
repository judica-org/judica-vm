use attest_messages::checkpoints::BitcoinCheckPoints;
use bitcoincore_rpc_async as rpc;
use rpc::{Client, RpcApi};
use sapio_bitcoin::BlockHash;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{sync::RwLock, task::JoinHandle};

mod util;
use crate::util::{AbstractResult, INFER_UNIT};

#[derive(Clone)]
pub struct BitcoinCheckPointCache {
    cache: Arc<RwLock<BitcoinCheckPoints>>,
    client: Arc<Client>,
    frequency: Duration,
    quit: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
}
impl BitcoinCheckPointCache {
    // Creates a new BitcoinCheckPointCache.
    // Default initialized if client cannot connect.
    pub async fn new(
        client: Arc<Client>,
        frequency: Option<Duration>,
        quit: Arc<AtomicBool>,
    ) -> Self {
        let new = fresh(&client, None)
            .await
            .unwrap_or_default()
            .unwrap_or_default();
        BitcoinCheckPointCache {
            cache: Arc::new(RwLock::new(new)),
            client,
            frequency: frequency.unwrap_or(Duration::from_secs(30)),
            quit,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn run_cache_service(&self) -> Option<JoinHandle<AbstractResult<()>>> {
        tracing::debug!("Bitcoin Client Starting...");
        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            == Ok(false)
        {
            let mut this = self.clone();
            Some(tokio::spawn(async move {
                while !this.quit.load(Ordering::Relaxed) {
                    tokio::time::sleep(this.frequency).await;
                    tracing::debug!("Attempting Cache Refresh");
                    this.refresh_cache().await;
                }
                this.running.store(false, Ordering::Relaxed);
                INFER_UNIT
            }))
        } else {
            tracing::error!("Bitcoin Client Already Started...");
            None
        }
    }
    pub async fn read_cache(&self) -> BitcoinCheckPoints {
        self.cache.read().await.clone()
    }
    async fn write_cache(&self, b: BitcoinCheckPoints) {
        let mut w = self.cache.write().await;
        *w = b;
    }
    async fn refresh_cache(&mut self) {
        let value_in_cache = self.read_cache().await.checkpoints[0].0;
        match fresh(&self.client, Some(value_in_cache)).await {
            Ok(Some(b)) => self.write_cache(b).await,
            Ok(None) => (),
            Err(_) => (),
        };
    }
}
async fn fresh(
    client: &Client,
    skip_if: Option<BlockHash>,
) -> rpc::Result<Option<BitcoinCheckPoints>> {
    loop {
        let h1 = client.get_best_block_hash().await?;
        if Some(h1) == skip_if {
            break Ok(None);
        }
        let info = client.get_block_header_info(&h1).await?;
        let height = info.height as u64;
        let h_six = client.get_block_hash(height - 6).await?;
        let h_day = client.get_block_hash(height - 144).await?;
        let h_week = client.get_block_hash(height - (144 * 7)).await?;
        let h_month = client.get_block_hash(height - (144 * 30)).await?;
        let h_check = client.get_best_block_hash().await?;
        if h_check != h1 {
            tracing::debug!("New Block Found During Refresh");
            continue;
        }
        break Ok(Some(BitcoinCheckPoints {
            checkpoints: [
                (h1, height as i64),
                (h_six, (height - 6) as i64),
                (h_day, (height - 144) as i64),
                (h_week, (height - (144 * 7)) as i64),
                (h_month, (height - (144 * 30)) as i64),
            ],
        }));
    }
}
