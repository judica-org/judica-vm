#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap, VecDeque,
    },
    error::Error,
    future::Future,
    sync::Arc,
    time::Duration,
};

use attest_database::{connection::MsgDB, generate_new_user, setup_db};
use attest_messages::{CanonicalEnvelopeHash, Envelope};
use mine_with_friends_board::{
    entity::EntityID,
    game::{
        game_move::{self, GameMove, Init, RegisterUser},
        GameBoard,
    },
    MoveEnvelope,
};
use sapio_bitcoin::{
    hashes::hex::ToHex,
    secp256k1::{All, Secp256k1},
    KeyPair, XOnlyPublicKey,
};
use schemars::{schema::RootSchema, schema_for};
use tauri::{
    async_runtime::{spawn, Mutex},
    State, Window,
};
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver},
        Notify, OnceCell,
    },
    time::sleep,
};

#[tauri::command]
async fn game_synchronizer(window: Window, game: State<'_, Game>) -> Result<(), ()> {
    loop {
        let game_s = {
            let g = game.inner().0.lock().await;
            serde_json::to_string(&*g)
        }
        .unwrap();
        window.emit("game-board", game_s).unwrap();
        game.1.notified().await;
    }
    Ok(())
}

#[tauri::command]
fn get_move_schema() -> RootSchema {
    schema_for!(GameMove)
}

#[tauri::command]
async fn list_my_users(
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
) -> Result<Vec<(XOnlyPublicKey, String)>, ()> {
    let msgdb = db.get().await.map_err(|_| ())?;
    let handle = msgdb.get_handle().await;
    let keys = handle.get_keymap().map_err(|_| ())?;
    let users = keys
        .keys()
        .map(|key| handle.locate_user(key))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ())?;
    let ret: Vec<(XOnlyPublicKey, String)> = users
        .iter()
        .zip(keys.keys())
        .map(|((a, b), k)| (k.clone(), b.clone()))
        .collect();
    Ok(ret)
}
#[tauri::command]
async fn make_new_user(
    nickname: String,
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
) -> Result<XOnlyPublicKey, Box<dyn Error>> {
    let (kp, next_nonce, genesis) = generate_new_user(secp.inner())?;
    let msgdb = db.get().await?;
    let handle = msgdb.get_handle().await;
    // TODO: Transaction?
    handle.insert_user_by_genesis_envelope(nickname, genesis.self_authenticate(secp.inner())?);
    let k = kp.public_key().x_only_public_key().0;
    handle.save_nonce_for_user_by_key(next_nonce, secp.inner(), k);
    Ok(k)
}

#[tauri::command]
async fn make_move_inner(
    game: State<'_, Game>,
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
    user: XOnlyPublicKey,
    nextMove: GameMove,
    from: EntityID,
) -> Result<(), ()> {
    let mut game = game.0.lock().await;
    let game = game.as_mut().ok_or(())?;
    let msgdb = db.get().await.map_err(|e| ())?;
    let v = serde_json::to_value(nextMove).map_err(|_| ())?;
    let handle = msgdb.get_handle().await;
    let keys = handle.get_keymap().map_err(|_| ())?;
    let sk = keys.get(&user).ok_or(())?;
    let keypair = KeyPair::from_secret_key(secp.inner(), sk);
    let msg = handle
        .wrap_message_in_envelope_for_user_by_key(v, &keypair, secp.inner())
        .ok()
        .ok_or(())?
        .ok()
        .ok_or(())?;
    let authenticated = msg.self_authenticate(secp.inner()).ok().ok_or(())?;
    let () = handle
        .try_insert_authenticated_envelope(authenticated)
        .ok()
        .ok_or(())?;
    return Ok::<(), ()>(());
    // game.play_inner(nextMove, from);
    // game.1.notify_waiters();
}

#[derive(Clone)]
struct Game(Arc<Mutex<Option<GameBoard>>>, Arc<Notify>);

// Safe to clone because MsgDB has Clone
#[derive(Clone)]
struct Database(OnceCell<MsgDB>);
impl Database {
    async fn get(&self) -> Result<MsgDB, Box<dyn Error>> {
        self.0
            .get_or_try_init(|| setup_db("attestations.mining-game"))
            .await
            .map(|v| v.clone())
    }
}
fn get_oracle_key() -> XOnlyPublicKey {
    todo!()
}
fn main() {
    let game = Arc::new(Mutex::new(Some(GameBoard::new())));
    let g = Game(game, Arc::new(Notify::new()));
    let db = Database(OnceCell::new());
    /// Goes through the oracles commitments in order
    let (sequencer_reader_task, output_envelope_hashes) = { start_sequencer(&db) };

    /// This task builds a HashMap of all unprocessed envelopes regularly
    let (shared_envelopes, notify_new_envelopes, shared_envelopes_task) =
        { start_envelope_db_fetcher(&db) };
    // Whenever new sequencing comes in, wait until they are all in the messages DB, and then drain them out for processing
    let (envelope_batcher, output_batches) = {
        start_envelope_batcher(
            shared_envelopes,
            notify_new_envelopes,
            output_envelope_hashes,
        )
    };
    // Run the deserialization of the inner message type to move sets in it's own thread so that we can process
    // moves in a pipeline as they get deserialized
    // TODO: We skip invalid moves? Should do something else?
    let (move_deserializer, output_moves) = { start_move_deserializer(output_batches) };
    // Play the moves one by one
    let game_task = {
        let g = g.clone();
        start_game(g, output_moves)
    };

    tauri::Builder::default()
        .setup(|app| Ok(()))
        .manage(Secp256k1::new())
        .manage(g.clone())
        .manage(db)
        .invoke_handler(tauri::generate_handler![
            game_synchronizer,
            get_move_schema,
            make_move_inner
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn start_sequencer(
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

fn start_envelope_db_fetcher(
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

fn start_envelope_batcher(
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

fn start_move_deserializer(
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

fn start_game(
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
