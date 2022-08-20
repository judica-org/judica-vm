use super::get_oracle_key;
use super::Database;
use super::Game;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::Envelope;
use mine_with_friends_board::MoveEnvelope;
use sapio_bitcoin::hashes::hex::ToHex;
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tauri;
use tauri::async_runtime::spawn;
use tauri::async_runtime::Mutex;
use tokio;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Notify;
use tokio::time::sleep;

pub(crate) fn start_sequencer(
    db: &Database,
) -> (
    tauri::async_runtime::JoinHandle<()>,
    tokio::sync::mpsc::UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>,
) {
    let (mut tx, mut rx) = unbounded_channel();
    let db = db.clone();
    let task = spawn(async move {
        let oracle_key = get_oracle_key();
        let mut count = 0;
        loop {
            let db = db.get().await.unwrap();
            'check: loop {
                let msg = {
                    let handle = db.get_handle().await;
                    handle.get_message_at_height_for_user(oracle_key, count)
                };
                match msg {
                    Ok(envelope) => {
                        match serde_json::from_value::<VecDeque<CanonicalEnvelopeHash>>(
                            envelope.msg,
                        ) {
                            Ok(v) => {
                                if tx.send(v).is_err() {
                                    return;
                                };
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

pub(crate) fn start_envelope_db_fetcher(
    db: &Database,
) -> (
    Arc<Mutex<HashMap<CanonicalEnvelopeHash, Envelope>>>,
    Arc<Notify>,
    tauri::async_runtime::JoinHandle<()>,
) {
    let shared_envelopes = Arc::new(Mutex::new(HashMap::<CanonicalEnvelopeHash, Envelope>::new()));
    let notify_new_envelopes = Arc::new(Notify::new());
    let envelopes = shared_envelopes.clone();
    let db = db.clone();
    let notify = notify_new_envelopes.clone();
    let task = spawn(async move {
        let mut newer = None;
        loop {
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
    });
    (shared_envelopes, notify_new_envelopes, task)
}

pub(crate) fn start_envelope_batcher(
    shared_envelopes: Arc<Mutex<HashMap<CanonicalEnvelopeHash, Envelope>>>,
    notify_new_envelopes: Arc<Notify>,
    mut input_envelope_hashers: UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>,
) -> (
    tauri::async_runtime::JoinHandle<()>,
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

pub(crate) fn start_move_deserializer(
    mut input_envelopes: tokio::sync::mpsc::UnboundedReceiver<Envelope>,
) -> (
    tauri::async_runtime::JoinHandle<()>,
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

pub(crate) fn start_game(
    g: Game,
    mut input_moves: UnboundedReceiver<(MoveEnvelope, String)>,
) -> tauri::async_runtime::JoinHandle<()> {
    let task = spawn(async move {
        while let Some((game_move, s)) = input_moves.recv().await {
            let mut game = g.0.lock().await;
            game.as_mut().unwrap().play(game_move, s);
            // TODO: Maybe notify less often?
            g.1.notify_waiters();
        }
    });
    task
}
