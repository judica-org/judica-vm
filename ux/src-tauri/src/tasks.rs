use super::Database;
use crate::Game;
use crate::GameStateInner;
use game_sequencer::OnlineDBFetcher;
use game_sequencer::Sequencer;
use sapio_bitcoin::hashes::hex::ToHex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio;
use tokio::spawn;
use tokio::sync::MutexGuard;
use tokio::task::JoinHandle;
use tracing::info;

/// Game Server Handle
pub struct GameServer {
    shutdown: Arc<AtomicBool>,
}

impl GameServer {
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed)
    }
    pub async fn await_shutdown() {
        // TODO: wait for all tasks to join
    }
    /// Start all Game Server functions
    pub async fn start(
        db: &Database,
        mut g_lock: MutexGuard<'_, Option<Game>>,
        g: GameStateInner,
    ) -> Result<(), &'static str> {
        if !std::ptr::eq(MutexGuard::mutex(&g_lock), &*g) {
            return Err("Must be same Mutex Passed in");
        }
        match g_lock.as_mut() {
            None => {
                return Err("No Game Available");
            }
            Some(game) => {
                if game.server.is_some() {
                    return Err("Game Already has a Server");
                }

                info!(key=?game.host_key, "Starting Game");
                let db = db.get().await.unwrap();
                let k = game.host_key;
                let shutdown: Arc<AtomicBool> = Default::default();
                let db_fetcher = OnlineDBFetcher::new(
                    shutdown.clone(),
                    Duration::from_secs(1),
                    Duration::from_secs(1),
                    k,
                    db,
                );
                let game_sequencer =
                    game_sequencer::Sequencer::new(shutdown.clone(), db_fetcher.clone());
                spawn(db_fetcher.run());
                spawn({
                    let game_sequencer = game_sequencer.clone();
                    game_sequencer.run()
                });
                let _game_task = {
                    let g = g;
                    start_game(shutdown.clone(), g, game_sequencer)
                };
                game.server = Some(Arc::new(GameServer { shutdown }));
            }
        }
        Ok(())
    }
}

// Play the moves one by one
pub(crate) fn start_game(
    _shutdown: Arc<AtomicBool>,
    g: GameStateInner,
    sequencer: Arc<Sequencer>,
) -> JoinHandle<()> {
    let task = spawn(async move {
        // TODO: Check which game the move is for?
        while let Some((game_move, s)) = sequencer.output_move().await {
            info!(move_ = ?game_move, "New Move Recieved");
            let mut game = g.lock().await;

            if let Some(game) = game.as_mut() {
                game.board.play(game_move, s.to_hex());
                // TODO: Maybe notify less often?
                game.should_notify.notify_waiters();
                info!("NOTIFYING Waiters of New State");
            }
        }
    });
    task
}
