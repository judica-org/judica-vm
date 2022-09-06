use super::Database;
use crate::Game;
use crate::GameStateInner;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::Envelope;
use game_host_messages::Peer;
use game_host_messages::{BroadcastByHost, Channelized};
use game_sequencer::OnlineDBFetcher;
use game_sequencer::Sequencer;
use mine_with_friends_board::MoveEnvelope;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::XOnlyPublicKey;
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tauri;
use tauri::async_runtime::Mutex;
use tokio;
use tokio::spawn;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::MutexGuard;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::sleep;

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
                let game_sequencer = game_sequencer::Sequencer::new(shutdown.clone(), db_fetcher);
                spawn({
                    let game_sequencer = game_sequencer.clone();
                    async move {
                        game_sequencer.run();
                    }
                });
                let game_task = {
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
    shutdown: Arc<AtomicBool>,
    g: GameStateInner,
    sequencer: Arc<Sequencer>,
) -> JoinHandle<()> {
    let task = spawn(async move {
        // TODO: Check which game the move is for?
        while let Some((game_move, s)) = sequencer.output_move().await {
            let mut game = g.lock().await;
            if let Some(game) = game.as_mut() {
                game.board.play(game_move, s.to_hex());
                // TODO: Maybe notify less often?
                game.should_notify.notify_waiters();
                println!("NOTIFYING");
            }
        }
    });
    task
}
