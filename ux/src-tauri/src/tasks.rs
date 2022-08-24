use super::Database;
use crate::Game;
use crate::GameStateInner;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::Envelope;
use game_host_messages::Peer;
use game_host_messages::{BroadcastByHost, Channelized};
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
                let k = game.host_key;
                let shutdown = Arc::new(AtomicBool::new(false));
                let (sequencer_reader_task, output_envelope_hashes) =
                    { start_sequencer(shutdown.clone(), k, db) };
                let (shared_envelopes, notify_new_envelopes, shared_envelopes_task) =
                    { start_envelope_db_fetcher(shutdown.clone(), db) };
                let (envelope_batcher, output_batches) = {
                    start_envelope_batcher(
                        shutdown.clone(),
                        shared_envelopes,
                        notify_new_envelopes,
                        output_envelope_hashes,
                    )
                };
                let (move_deserializer, output_moves) =
                    { start_move_deserializer(shutdown.clone(), output_batches) };
                let game_task = {
                    let g = g;
                    start_game(shutdown.clone(), g, output_moves)
                };
                game.server = Some(Arc::new(GameServer { shutdown }));
            }
        }
        Ok(())
    }
}

/// Goes through the oracles commitments in order
pub(crate) fn start_sequencer(
    shutdown: Arc<AtomicBool>,
    oracle_key: XOnlyPublicKey,
    db: &Database,
) -> (
    JoinHandle<()>,
    tokio::sync::mpsc::UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>,
) {
    let (mut tx, mut rx) = unbounded_channel();
    let db = db.clone();
    let task = spawn(async move {
        let mut count = 0;
        while !shutdown.load(Ordering::Relaxed) {
            let db = db.get().await.unwrap();
            'check: while !shutdown.load(Ordering::Relaxed) {
                let msg = {
                    let handle = db.get_handle().await;
                    handle.get_message_at_height_for_user(oracle_key, count)
                };
                match msg {
                    Ok(envelope) => {
                        match serde_json::from_value::<Channelized<BroadcastByHost>>(envelope.msg) {
                            Ok(v) => {
                                match v.data {
                                    BroadcastByHost::Sequence(s) => {
                                        if tx.send(s).is_err() {
                                            return;
                                        };
                                    }
                                    BroadcastByHost::NewPeer(Peer { tor, port }) => {
                                        let handle = db.get_handle().await;
                                        // idempotent
                                        handle.insert_hidden_service(tor, port);
                                    }
                                }
                                count += 1;
                            }
                            Err(_) => {
                                return;
                            }
                        }
                        break 'check;
                    }
                    Err(_) => {
                        sleep(Duration::from_secs(10)).await;
                    }
                }
            }
        }
    });
    (task, rx)
}

/// This task builds a HashMap of all unprocessed envelopes regularly
pub(crate) fn start_envelope_db_fetcher(
    shutdown: Arc<AtomicBool>,
    db: &Database,
) -> (
    Arc<Mutex<HashMap<CanonicalEnvelopeHash, Envelope>>>,
    Arc<Notify>,
    JoinHandle<()>,
) {
    let shared_envelopes = Arc::new(Mutex::new(HashMap::<CanonicalEnvelopeHash, Envelope>::new()));
    let notify_new_envelopes = Arc::new(Notify::new());
    let envelopes = shared_envelopes.clone();
    let db = db.clone();
    let notify = notify_new_envelopes.clone();
    let task = spawn(async move {
        let mut newer = None;
        while !shutdown.load(Ordering::Relaxed) {
            let newer_before = newer;
            {
                let db = db.get().await.unwrap();
                let handle = db.get_handle().await;
                let mut env = envelopes.lock().await;
                handle.get_all_messages_collect_into_inconsistent(&mut newer, &mut env);
            }

            if newer_before != newer {
                notify.notify_waiters();
            }
            sleep(Duration::from_secs(10)).await;
        }
        notify.notify_waiters();
    });
    (shared_envelopes, notify_new_envelopes, task)
}

// Whenever new sequencing comes in, wait until they are all in the messages DB, and then drain them out for processing
pub(crate) fn start_envelope_batcher(
    shutdown: Arc<AtomicBool>,
    shared_envelopes: Arc<Mutex<HashMap<CanonicalEnvelopeHash, Envelope>>>,
    notify_new_envelopes: Arc<Notify>,
    mut input_envelope_hashers: UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>,
) -> (
    JoinHandle<()>,
    tokio::sync::mpsc::UnboundedReceiver<Envelope>,
) {
    let (mut tx_envelopes_to_process, mut rx_envelopes_to_process) = unbounded_channel();
    let envelopes = shared_envelopes.clone();
    let notify = notify_new_envelopes.clone();
    let task = spawn(async move {
        while let Some(mut envelope_hashes) = input_envelope_hashers.recv().await {
            let mut should_wait = None;
            'wait_for_new: while envelope_hashes.len() != 0 {
                if let Some(n) = should_wait.take() {
                    // register for notification, then drop lock, then wait
                    n.await;
                    // if we got woken up because of shutdown, shut down.
                    if shutdown.load(Ordering::Relaxed) {
                        return;
                    }
                }
                let mut envs = envelopes.lock().await;
                while let Some(envelope) = envelope_hashes.pop_front() {
                    match envs.entry(envelope) {
                        Occupied(e) => {
                            // TODO: Batch size
                            tx_envelopes_to_process.send(e.remove());
                        }
                        Vacant(k) => {
                            envelope_hashes.push_front(k.into_key());
                            should_wait.insert(notify.notified());
                            break 'wait_for_new;
                        }
                    }
                }
            }
        }
    });
    (task, rx_envelopes_to_process)
}

// Run the deserialization of the inner message type to move sets in it's own thread so that we can process
// moves in a pipeline as they get deserialized
// TODO: We skip invalid moves? Should do something else?
pub(crate) fn start_move_deserializer(
    shutdown: Arc<AtomicBool>,
    mut input_envelopes: tokio::sync::mpsc::UnboundedReceiver<Envelope>,
) -> (
    JoinHandle<()>,
    tokio::sync::mpsc::UnboundedReceiver<(MoveEnvelope, String)>,
) {
    let (mut tx2, mut rx2) = unbounded_channel();
    let task = spawn(async move {
        while let Some(envelope) = input_envelopes.recv().await {
            let r_game_move = serde_json::from_value(envelope.msg);
            match r_game_move {
                Ok(game_move) => {
                    if tx2.send((game_move, envelope.header.key.to_hex())).is_err() {
                        return;
                    }
                }
                Err(_) => {}
            }
        }
    });
    (task, rx2)
}

// Play the moves one by one
pub(crate) fn start_game(
    shutdown: Arc<AtomicBool>,
    g: GameStateInner,
    mut input_moves: UnboundedReceiver<(MoveEnvelope, String)>,
) -> JoinHandle<()> {
    let task = spawn(async move {
        // TODO: Check which game the move is for?
        while let Some((game_move, s)) = input_moves.recv().await {
            let mut game = g.lock().await;
            if let Some(game) = game.as_mut() {
                game.board.play(game_move, s);
                // TODO: Maybe notify less often?
                game.should_notify.notify_waiters();
                println!("NOTIFYING");
            }
        }
    });
    task
}
