#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use attest_database::{
    connection::MsgDB, db_handle::create::TipControl, generate_new_user, setup_db,
};
use mine_with_friends_board::{
    entity::EntityID,
    game::{
        game_move::{GameMove, Init, Trade},
        GameBoard,
    },
    tokens::{
        token_swap::{TradingPairID, UXMaterialsPriceData},
        TokenPointer,
    },
};
use sapio_bitcoin::{
    secp256k1::{All, Secp256k1},
    KeyPair, XOnlyPublicKey,
};
use schemars::{schema::RootSchema, schema_for};
use std::{error::Error, path::PathBuf, sync::Arc};
use tasks::GameServer;
use tauri::{async_runtime::Mutex, State, Window};
use tokio::{
    spawn,
    sync::{futures::Notified, Notify, OnceCell},
};
mod tasks;

#[tauri::command]
async fn game_synchronizer(
    window: Window,
    s: GameState<'_>,
    d: State<'_, Database>,
) -> Result<(), ()> {
    println!("Registering");
    loop {
        // No Idea why the borrow checker likes this, but it seems to be the case
        // that because the notified needs to live inside the async state machine
        // hapily, giving a stable reference to it tricks the compiler into thinking
        // that the lifetime is 'static and we can successfully wait on it outside
        // the lock.
        let mut arc_cheat = None;
        let (gamestring, wait_on, key) = {
            let game = s.inner().lock().await;
            let s = game
                .as_ref()
                .map(|g| serde_json::to_string(&g.board).unwrap_or("null".into()))
                .unwrap_or("null".into());
            arc_cheat = game.as_ref().map(|g: &Game| g.should_notify.clone());
            let w: Option<Notified> = arc_cheat.as_ref().map(|x| x.notified());
            (s, w, game.as_ref().map(|g| g.host_key))
        };
        let (appName, prefix) = {
            let l = d.inner().state.lock().await;
            if let Some(g) = l.as_ref() {
                (g.name.clone(), g.prefix.clone())
            } else {
                ("".into(), None)
            }
        };
        // Attempt to get data to show prices
        let raw_price_data = {
            let mut game = s.inner().lock().await;
            let p = game
                .as_mut()
                .map(|g| g.board.get_ux_materials_prices())
                .unwrap_or(Ok(Vec::new()))
                .unwrap();
            p
        };

        println!("Emitting!");

        window.emit("host-key", key).unwrap();
        window.emit("db-connection", (appName, prefix)).unwrap();
        window.emit("game-board", gamestring).unwrap();
        window.emit("materials-price-data", raw_price_data).unwrap();
        if let Some(w) = wait_on {
            w.await;
        } else {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}

#[tauri::command]
fn get_move_schema() -> RootSchema {
    schema_for!(GameMove)
}

#[tauri::command]
fn get_materials_schema() -> RootSchema {
    schema_for!(Trade)
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
    let (kp, next_nonce, genesis) = generate_new_user(secp.inner(), None::<()>)?;
    let msgdb = db.get().await?;
    let mut handle = msgdb.get_handle().await;
    // TODO: Transaction?
    handle.insert_user_by_genesis_envelope(nickname, genesis.self_authenticate(secp.inner())?);
    let k = kp.public_key().x_only_public_key().0;
    handle.save_nonce_for_user_by_key(next_nonce, secp.inner(), k);
    Ok(k)
}

#[tauri::command]
async fn make_move_inner(
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
    user: XOnlyPublicKey,
    nextMove: GameMove,
    from: EntityID,
) -> Result<(), ()> {
    let msgdb = db.get().await.map_err(|e| ())?;
    let v = ruma_serde::to_canonical_value(nextMove).map_err(|_| ())?;
    let mut handle = msgdb.get_handle().await;
    let keys = handle.get_keymap().map_err(|_| ())?;
    let sk = keys.get(&user).ok_or(())?;
    let keypair = KeyPair::from_secret_key(secp.inner(), sk);
    // TODO: Runa tipcache
    let msg = handle
        .wrap_message_in_envelope_for_user_by_key(
            v,
            &keypair,
            secp.inner(),
            None,
            None,
            TipControl::AllTips,
        )
        .ok()
        .ok_or(())?
        .ok()
        .ok_or(())?;
    let authenticated = msg.self_authenticate(secp.inner()).ok().ok_or(())?;
    let _ = handle
        .try_insert_authenticated_envelope(authenticated)
        .ok()
        .ok_or(())?;
    return Ok::<(), ()>(());
}

#[tauri::command]
async fn switch_to_game(
    db: State<'_, Database>,
    game: GameState<'_>,
    key: XOnlyPublicKey,
) -> Result<(), ()> {
    let db = db.inner().clone();
    let game = game.inner().clone();
    spawn(async move {
        let game2 = game.clone();
        let mut g = game2.lock().await;
        g.as_mut()
            .map(|game| game.server.as_ref().map(|s| s.shutdown()));
        let mut new_game = Game {
            board: GameBoard::new(),
            should_notify: Arc::new(Notify::new()),
            host_key: key,
            server: None,
        };
        new_game
            .board
            .play_inner(GameMove::Init(Init()), EntityID(0))
            .unwrap();
        *g = Some(new_game);
        GameServer::start(&db, g, game).await?;
        Ok::<(), &'static str>(())
    });
    Ok(())
}

#[tauri::command]
async fn switch_to_db(
    window: Window,
    db: State<'_, Database>,
    appName: String,
    prefix: Option<PathBuf>,
) -> Result<(), ()> {
    let res = db.connect(&appName, prefix.clone()).await.map_err(|_| ());

    res
}

pub struct Game {
    board: GameBoard,
    should_notify: Arc<Notify>,
    host_key: XOnlyPublicKey,
    server: Option<Arc<GameServer>>,
}

type GameStateInner = Arc<Mutex<Option<Game>>>;
type GameState<'a> = State<'a, GameStateInner>;

// Safe to clone because MsgDB has Clone
#[derive(Clone)]
pub struct Database {
    state: Arc<Mutex<Option<DatabaseInner>>>,
}

pub struct DatabaseInner {
    db: MsgDB,
    name: String,
    prefix: Option<PathBuf>,
}
impl Database {
    async fn get(&self) -> Result<MsgDB, Box<dyn Error>> {
        Ok(self
            .state
            .lock()
            .await
            .as_ref()
            .ok_or("No Database Connection")?
            .db
            .clone())
    }
    async fn connect(&self, appname: &str, prefix: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
        let mut g = self.state.lock().await;
        *g = Some(DatabaseInner {
            db: setup_db(appname, prefix.clone()).await?,
            name: appname.to_owned(),
            prefix: prefix.clone(),
        });
        Ok(())
    }
}
fn main() {
    let game = GameStateInner::new(Mutex::new(None));
    let db = Database {
        state: Arc::new(Mutex::new(None)),
    };
    tauri::Builder::default()
        .setup(|app| Ok(()))
        .manage(Secp256k1::new())
        .manage(game.clone())
        .manage(db)
        .invoke_handler(tauri::generate_handler![
            game_synchronizer,
            get_move_schema,
            get_materials_schema,
            make_move_inner,
            switch_to_game,
            switch_to_db,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
