#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{error::Error, future::Future, sync::Arc, time::Duration};

use attest_database::{connection::MsgDB, generate_new_user, setup_db};
use attest_messages::CanonicalEnvelopeHash;
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
    sync::{mpsc::unbounded_channel, Notify, OnceCell},
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
    let (mut tx, mut rx) = unbounded_channel();
    {
        let db = db.clone();
        spawn(async move {
            let oracle_key = get_oracle_key();
            let mut count = 0;
            loop {
                let db = db.get().await.unwrap();
                let handle = db.get_handle().await;
                'check: loop {
                    let msg = handle.get_message_at_height_for_user(oracle_key, count);
                    match msg {
                        Ok(envelope) => {
                            match serde_json::from_value::<Vec<CanonicalEnvelopeHash>>(envelope.msg)
                            {
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
    }
    {
        let db = db.clone();
        let g = g.clone();
        spawn(async move {
            let db = db.get().await.unwrap();
            let handle = db.get_handle().await;
            let envelopes = rx.recv().await;
            match envelopes {
                Some(v) => {
                    let msgs = handle.messages_by_hash(v.iter()).unwrap();
                    for envelope in msgs {
                        let mut game = g.0.lock().await;
                        let game = game.as_mut().unwrap();
                        let r_game_move = serde_json::from_value(envelope.msg);
                        match r_game_move {
                            Ok(game_move) => {
                                game.play(game_move, envelope.header.key.to_hex());
                                // TODO: Maybe notify less often?
                                g.1.notify_waiters();
                            }
                            Err(_) => {}
                        }
                        break;
                    }
                }
                None => todo!(),
            }
        });
    }

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
