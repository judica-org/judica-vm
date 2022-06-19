use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use bitcoincore_rpc_async as rpc;
use rpc::RpcApi;
use sapio_bitcoin::BlockHash;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{Mutex, RwLock},
    task::JoinHandle,
    time::Interval,
};

use crate::util::{AbstractResult, INFER_UNIT};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BitcoinCheckPoints {
    /// whatever tip hash we've seen recently present if changed where it should
    /// be roughly:
    ///
    /// - Index 0: most recent
    /// - Index 1: 6 ago
    /// - Index 2: 144 ago
    /// - Index 3: 144*7 ago
    /// - Index 4: Arbitrary
    ///
    /// By including these 5, we guarantee a proof of "afterness" withing
    /// reasonable bounds.
    ///
    /// If the hashes are unknown at lower indexes (because of reorg), do not
    /// treat as an error.
    ///
    /// The relative bound between blocks is not checked.
    ///
    /// Even if the hashes haven't changed, we still log them.
    ///
    /// Note that we may already transitively commit to these (or later)
    /// checkpoints via other commitments in the header.
    pub checkpoints: [(BlockHash, i64); 5],
}

impl Default for BitcoinCheckPoints {
    fn default() -> Self {
        Self {
            checkpoints: [(Default::default(), -1); 5],
        }
    }
}

#[derive(Clone)]
pub struct BitcoinCheckPointCache {
    cache: Arc<RwLock<BitcoinCheckPoints>>,
    client: Arc<rpc::Client>,
    frequency: Duration,
    quit: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
}
impl BitcoinCheckPointCache {
    pub async fn new(
        client: Arc<rpc::Client>,
        frequency: Option<Duration>,
        quit: Arc<AtomicBool>,
    ) -> Result<Option<Self>, rpc::Error> {
        let new = BitcoinCheckPoints::fresh(&client, None).await?;
        Ok(new.map(|entry| BitcoinCheckPointCache {
            cache: Arc::new(RwLock::new(entry)),
            client,
            frequency: frequency.unwrap_or(Duration::from_secs(30)),
            quit,
            running: Arc::new(AtomicBool::new(false)),
        }))
    }

    pub async fn run_cache_service(&self) -> Option<JoinHandle<AbstractResult<()>>> {
        if !self.running.compare_and_swap(false, true, Ordering::SeqCst) {
            let mut this = self.clone();
            Some(tokio::spawn(async move {
                while !this.quit.load(Ordering::Relaxed) {
                    tokio::time::sleep(this.frequency).await;
                    this.refresh_cache().await;
                }
                this.running.store(false, Ordering::Relaxed);
                INFER_UNIT
            }))
        } else {
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
        match BitcoinCheckPoints::fresh(&self.client, Some(value_in_cache)).await {
            Ok(Some(b)) => self.write_cache(b).await,
            Ok(None) => (),
            Err(_) => (),
        };
    }
}
impl BitcoinCheckPoints {
    async fn fresh(
        client: &rpc::Client,
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
}
