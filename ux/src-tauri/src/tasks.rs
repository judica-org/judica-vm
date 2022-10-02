use super::Database;
use crate::Game;
use crate::GameStateInner;
use crate::SigningKeyInner;
use game_sequencer::OnlineDBFetcher;
use game_sequencer::Sequencer;
use mine_with_friends_board::entity::EntityID;
use mine_with_friends_board::game::game_move::GameMove;
use mine_with_friends_board::game::MoveRejectReason;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::secp256k1::All;
use sapio_bitcoin::secp256k1::Secp256k1;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::spawn;
use tokio::sync::MutexGuard;
use tokio::task::JoinHandle;
use tracing::debug;
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
        secp: Arc<Secp256k1<All>>,
        signing_key: SigningKeyInner,
        database: Database,
        mut g_lock: MutexGuard<'_, Option<Game>>,
        g: GameStateInner,
    ) -> Result<(), &'static str> {
        tracing::trace!("Starting Game Server");
        if !std::ptr::eq(MutexGuard::mutex(&g_lock), &*g) {
            return Err("Must be same Mutex Passed in");
        }
        match g_lock.as_mut() {
            None => {
                tracing::trace!("No Such Game");
                return Err("No Game Available");
            }
            Some(game) => {
                if game.server.is_some() {
                    tracing::trace!("Game Already Started");
                    return Err("Game Already has a Server");
                }

                info!(key=?game.host_key, "Starting Game");
                let db = database.get().await.unwrap();
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
                    game_sequencer.0.run()
                });
                // Can be re-enabled whenever, but disable it after game is finished so we don't spam the logs...
                let heartbeat_enable = Arc::new(AtomicBool::new(true));
                let game_task = {
                    let g = g;
                    start_game(
                        shutdown.clone(),
                        g,
                        game_sequencer,
                        heartbeat_enable.clone(),
                    )
                };
                spawn({
                    let database = database.clone();
                    let shutdown = shutdown.clone();
                    let secp = secp;
                    let signing_key = signing_key.clone();
                    async move {
                        let mut t = tokio::time::interval(Duration::from_millis(5000));
                        loop {
                            let a = t.tick().await;
                            if shutdown.load(Ordering::Relaxed) {
                                break;
                            }
                            if heartbeat_enable.load(Ordering::Relaxed) {
                                tracing::trace!("Game Heartbeat!");
                                crate::commands::modify::make_move_inner_inner(
                                    secp.clone(),
                                    database.clone(),
                                    signing_key.clone(),
                                    GameMove::Heartbeat(
                                        mine_with_friends_board::game::game_move::Heartbeat(),
                                    ),
                                )
                                .await
                                .map_err(|e| tracing::trace!(err=?e, "Game Heartbeat!"));
                            }
                        }
                    }
                });
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
    sequencer: Sequencer,
    heartbeat_enable: Arc<AtomicBool>,
) -> JoinHandle<()> {
    spawn(async move {
        // TODO: Check which game the move is for?
        while let Some((game_move, s)) = sequencer.output_move().await {
            info!(move_ = ?game_move, "New Move Recieved");
            let mut game = g.lock().await;

            if let Some(game) = game.as_mut() {
                if let Err(err) = game.board.play(game_move, s.to_hex()) {
                    match err {
                        MoveRejectReason::NoSuchUser => {}
                        MoveRejectReason::GameIsFinished(_) => {
                            heartbeat_enable.store(false, Ordering::Relaxed);
                        }
                        MoveRejectReason::MoveSanitizationError(_) => {}
                        MoveRejectReason::TradeRejected(_) => {}
                    }
                    debug!(reason=?err, "Rejected Move");
                } else {
                    // TODO: Maybe notify less often?
                    game.should_notify.notify_waiters();
                    info!("NOTIFYING Waiters of New State");
                }
            }
        }
    })
}
