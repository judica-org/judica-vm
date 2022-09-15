#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use attest_database::{
    connection::MsgDB,
    db_handle::{create::TipControl, MsgDBHandle},
    generate_new_user, setup_db,
};
use game_host_messages::{BroadcastByHost, Channelized};
use mine_with_friends_board::{
    entity::EntityID,
    game::{
        game_move::{Chat, GameMove, Heartbeat, PurchaseNFT, Trade},
        GameBoard,
    },
    nfts::{sale::UXForSaleList, NftPtr, UXNFTRegistry, UXPlantData},
    sanitize::Unsanitized,
    tokens::{
        token_swap::{TradingPairID, UXMaterialsPriceData},
        TokenPointer,
    },
    MoveEnvelope,
};
use sapio_bitcoin::{
    hashes::hex::ToHex,
    secp256k1::{All, Secp256k1},
    KeyPair, XOnlyPublicKey,
};
use schemars::{schema::RootSchema, schema_for};
use std::{collections::BTreeMap, error::Error, path::PathBuf, sync::Arc};
use tasks::GameServer;
use tauri::{async_runtime::Mutex, State, Window};
use tokio::{
    spawn,
    sync::{futures::Notified, Notify, OnceCell},
};
use tracing::{info, warn};
mod tasks;

struct PrintOnDrop(String);
impl Drop for PrintOnDrop {
    fn drop(&mut self) {
        warn!("{}", self.0);
    }
}

#[tauri::command]
async fn game_synchronizer(
    window: Window,
    s: GameState<'_>,
    d: State<'_, Database>,
    signing_key: State<'_, SigningKeyInner>,
) -> Result<(), ()> {
    info!("Registering Window for State Updates");
    let p = PrintOnDrop("Registration Canceled".into());
    loop {
        // No Idea why the borrow checker likes this, but it seems to be the case
        // that because the notified needs to live inside the async state machine
        // hapily, giving a stable reference to it tricks the compiler into thinking
        // that the lifetime is 'static and we can successfully wait on it outside
        // the lock.
        let mut arc_cheat = None;
        let (gamestring, wait_on, key, chat_log) = {
            let game = s.inner().lock().await;
            let s = game
                .as_ref()
                .map(|g| serde_json::to_string(&g.board).unwrap_or("null".into()))
                .unwrap_or("null".into());
            arc_cheat = game.as_ref().map(|g: &Game| g.should_notify.clone());
            let w: Option<Notified> = arc_cheat.as_ref().map(|x| x.notified());
            let chat_log = game
                .as_ref()
                .map(|g| g.board.get_ux_chat_log())
                .unwrap_or_default();
            (s, w, game.as_ref().map(|g| g.host_key), chat_log)
        };
        let (appName, prefix, list_of_chains, user_keys) = {
            let l = d.inner().state.lock().await;
            if let Some(g) = l.as_ref() {
                let mut handle = g.db.get_handle().await;
                let v = handle.get_all_users().map_err(|_| ())?;
                let keys: Vec<XOnlyPublicKey> = handle.get_keymap().unwrap().into_keys().collect();
                (g.name.clone(), g.prefix.clone(), v, keys)
            } else {
                ("".into(), None, vec![], vec![])
            }
        };

        // TODO: move these under a single held game lock
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

        let power_plants = {
            let mut game = s.inner().lock().await;
            let plants: Vec<(NftPtr, UXPlantData)> = game
                .as_mut()
                .map(|g| g.board.get_ux_power_plant_data())
                .unwrap_or_else(|| Vec::new());
            plants
        };

        let listings = {
            let game = s.inner().lock().await;
            let listings = game
                .as_ref()
                .map(|g| g.board.get_ux_energy_market())
                .unwrap_or(Ok(UXForSaleList {
                    listings: Vec::new(),
                }))
                .unwrap();
            listings
        };

        info!("Emitting State Updates");
        window.emit("available-sequencers", list_of_chains);
        let signing_key: Option<_> = *signing_key.inner().lock().await;
        window.emit("chat-log", chat_log);
        window.emit("signing-key", signing_key);
        window.emit("host-key", key).unwrap();
        window.emit("user-keys", user_keys).unwrap();
        window.emit("db-connection", (appName, prefix)).unwrap();
        window.emit("game-board", gamestring).unwrap();
        window.emit("materials-price-data", raw_price_data).unwrap();
        window.emit("power-plants", power_plants).unwrap();
        window.emit("energy-exchange", listings.listings).unwrap();
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
fn get_purchase_schema() -> RootSchema {
    schema_for!(PurchaseNFT)
}

// get transfer_token schema, get list of tokens, and make individual components around each
// make sure everything is consistent state wise.
// wrap in game move when you handleSubmit

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

trait ErrToString<E> {
    fn err_to_string(self) -> Result<E, String>;
}
impl<E, T: std::fmt::Debug> ErrToString<E> for Result<E, T> {
    fn err_to_string(self) -> Result<E, String> {
        self.map_err(|e| format!("{:?}", e))
    }
}

#[tauri::command]
async fn make_new_chain(
    nickname: String,
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
) -> Result<String, String> {
    let (kp, next_nonce, genesis) = generate_new_user(
        secp.inner(),
        Some(MoveEnvelope {
            d: Unsanitized(GameMove::Heartbeat(Heartbeat())),
            sequence: 0,
            /// The player who is making the move, myst be figured out somewhere...
            time: attest_util::now() as u64,
        }),
    )
    .err_to_string()?;
    let msgdb = db.get().await.err_to_string()?;
    let mut handle = msgdb.get_handle().await;
    // TODO: Transaction?
    handle.save_keypair(kp).err_to_string()?;
    let k = kp.public_key().x_only_public_key().0;
    handle.save_nonce_for_user_by_key(next_nonce, secp.inner(), k);
    handle.insert_user_by_genesis_envelope(
        nickname,
        genesis.self_authenticate(secp.inner()).err_to_string()?,
    );
    Ok(k.to_hex())
}

#[tauri::command]
async fn send_chat(
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
    sk: State<'_, SigningKeyInner>,
    chat: String,
) -> Result<(), ()> {
    make_move_inner(secp, db, sk, GameMove::from(Chat(chat)), EntityID(0)).await
}

#[tauri::command]
async fn make_move_inner(
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
    sk: State<'_, SigningKeyInner>,
    nextMove: GameMove,
    from: EntityID,
) -> Result<(), ()> {
    let xpubkey = sk.inner().lock().await.ok_or(())?;
    let msgdb = db.get().await.map_err(|e| ())?;
    let mut handle = msgdb.get_handle().await;
    let tip = handle.get_tip_for_user_by_key(xpubkey).map_err(|_| ())?;
    let last: MoveEnvelope = serde_json::from_value(tip.msg().to_owned().into()).map_err(|_| ())?;
    let mve = MoveEnvelope {
        d: Unsanitized(nextMove),
        sequence: last.sequence + 1,
        time: attest_util::now() as u64,
    };
    let v = ruma_serde::to_canonical_value(mve).map_err(|_| ())?;
    let keys = handle.get_keymap().map_err(|_| ())?;
    let sk = keys.get(&xpubkey).ok_or(())?;
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
        let game_setup = {
            let db = db.state.lock().await;
            let db: &DatabaseInner = db.as_ref().ok_or("No Database Set Up")?;
            let handle = db.db.get_handle().await;
            let genesis = handle
                .get_message_at_height_for_user(key, 0)
                .map_err(|_| "No Genesis found for selected Key")?;
            if let Ok(Channelized {
                data: BroadcastByHost::GameSetup(g),
                channel: _,
            }) = serde_json::from_value(genesis.msg().to_owned().into())
            {
                g
            } else {
                return Err("First Message was not a GameSetup");
            }
        };

        let game2 = game.clone();
        let mut g = game2.lock().await;
        g.as_mut()
            .map(|game| game.server.as_ref().map(|s| s.shutdown()));
        let mut new_game = Game {
            board: GameBoard::new(game_setup),
            should_notify: Arc::new(Notify::new()),
            host_key: key,
            server: None,
        };
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

#[tauri::command]
async fn set_signing_key(
    s: GameState<'_>,
    selected: Option<XOnlyPublicKey>,
    sk: State<'_, SigningKeyInner>,
) -> Result<(), ()> {
    {
        let mut l = sk.inner().lock().await;
        *l = selected;
    }
    {
        let mut l = s.lock().await;
        l.as_ref().map(|g| g.should_notify.notify_one());
    }

    Ok(())
}

pub struct Game {
    board: GameBoard,
    should_notify: Arc<Notify>,
    host_key: XOnlyPublicKey,
    server: Option<Arc<GameServer>>,
}

type GameStateInner = Arc<Mutex<Option<Game>>>;
type GameState<'a> = State<'a, GameStateInner>;

type SigningKeyInner = Arc<Mutex<Option<XOnlyPublicKey>>>;

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
            db: setup_db(&format!("attestations.{}", appname), prefix.clone()).await?,
            name: appname.to_owned(),
            prefix: prefix.clone(),
        });
        Ok(())
    }
}
fn main() {
    tracing_subscriber::fmt::init();
    let game = GameStateInner::new(Mutex::new(None));
    let db = Database {
        state: Arc::new(Mutex::new(None)),
    };
    let sk = SigningKeyInner::new(Mutex::new(None));
    tauri::Builder::default()
        .setup(|app| Ok(()))
        .manage(Secp256k1::new())
        .manage(game.clone())
        .manage(db)
        .manage(sk)
        .invoke_handler(tauri::generate_handler![
            game_synchronizer,
            get_move_schema,
            get_materials_schema,
            get_purchase_schema,
            make_move_inner,
            switch_to_game,
            switch_to_db,
            set_signing_key,
            send_chat,
            make_new_chain
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
