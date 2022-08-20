#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{error::Error, future::Future, sync::Arc, time::Duration};

use attest_database::{connection::MsgDB, generate_new_user, setup_db};
use mine_with_friends_board::{
    entity::EntityID,
    game::{
        game_move::{self, GameMove, Init, RegisterUser},
        GameBoard,
    },
};
use sapio_bitcoin::{
    hashes::hex::ToHex,
    secp256k1::{All, Secp256k1},
    KeyPair, XOnlyPublicKey,
};
use schemars::{schema::RootSchema, schema_for};
use tauri::{async_runtime::Mutex, State, Window};
use tokio::{
    sync::{Notify, OnceCell},
    time::sleep,
};
mod tasks;

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
    let (sequencer_reader_task, output_envelope_hashes) = { tasks::start_sequencer(&db) };

    /// This task builds a HashMap of all unprocessed envelopes regularly
    let (shared_envelopes, notify_new_envelopes, shared_envelopes_task) =
        { tasks::start_envelope_db_fetcher(&db) };
    // Whenever new sequencing comes in, wait until they are all in the messages DB, and then drain them out for processing
    let (envelope_batcher, output_batches) = {
        tasks::start_envelope_batcher(
            shared_envelopes,
            notify_new_envelopes,
            output_envelope_hashes,
        )
    };
    // Run the deserialization of the inner message type to move sets in it's own thread so that we can process
    // moves in a pipeline as they get deserialized
    // TODO: We skip invalid moves? Should do something else?
    let (move_deserializer, output_moves) = { tasks::start_move_deserializer(output_batches) };
    // Play the moves one by one
    let game_task = {
        let g = g.clone();
        tasks::start_game(g, output_moves)
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
