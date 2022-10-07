#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use crate::config::Globals;
use attest_database::{connection::MsgDB, setup_db};
use commands::bindings::HANDLER;
use config::Config;
use game_host_messages::JoinCode;
use mine_with_friends_board::game::GameBoard;
use sapio_bitcoin::{secp256k1::Secp256k1, XOnlyPublicKey};
use serde::Deserialize;
use serde::Serialize;
use tor::start;
use std::{error::Error, path::PathBuf, sync::Arc};
use tasks::GameServer;
use tauri::{async_runtime::Mutex, window, Manager, State};
use tokio::sync::Notify;
use tor::GameHost;
use tor::TorClient;
use tracing::info;
use tracing::warn;

mod commands;
mod config;
mod tasks;
mod tor;

struct PrintOnDrop(String);
impl Drop for PrintOnDrop {
    fn drop(&mut self) {
        warn!("{}", self.0);
    }
}

pub struct Game {
    board: GameBoard,
    should_notify: Arc<Notify>,
    host_key: XOnlyPublicKey,
    server: Option<Arc<GameServer>>,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct Pending {
    pub join_code: game_host_messages::JoinCode,
    pub password: Option<JoinCode>,
}

pub enum GameInitState {
    Game(Game),
    Pending(Pending),
    None,
}
impl GameInitState {
    pub fn is_none(&self) -> bool {
        matches!(self, GameInitState::None)
    }
    pub fn game_mut(&mut self) -> Option<&mut Game> {
        match self {
            GameInitState::Game(g) => Some(g),
            GameInitState::Pending(_) | GameInitState::None => None,
        }
    }
    pub fn game_opt(&self) -> Option<&Game> {
        match self {
            GameInitState::Game(g) => Some(g),
            GameInitState::Pending(_) | GameInitState::None => None,
        }
    }
}

type GameStateInner = Arc<Mutex<GameInitState>>;
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

#[derive(Serialize, Deserialize)]
pub struct DBSelector {
    pub appname: String,
    pub prefix: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    let game = GameStateInner::new(Mutex::new(GameInitState::None));
    let db = Database {
        state: Arc::new(Mutex::new(None)),
    };
    let db_for_setup = db.clone();
    let sk = SigningKeyInner::new(Mutex::new(None));
    let globals: Arc<Globals> = if let Ok(s) = std::env::var("MASTERMINE_CONFIG") {
        Globals::new(serde_json::from_str(&s).map_err(|_| "Invalid Config")?)
    } else {
        return Err("No Config")?;
    };
    tauri::Builder::default()
        .manage(Arc::new(Secp256k1::new()))
        .manage(game)
        .manage(db)
        .manage(sk)
        .manage(globals.clone())
        .manage(Arc::new(Mutex::new(None::<GameHost>)))
        .setup(move |app| {
            let app_handle = app.app_handle();
            app.listen_global("globe-click", move |e| {
                info!("globe-click payload {:?}:", e.payload().unwrap());
                app_handle
                    .emit_all("globe-location", e.payload().unwrap())
                    .unwrap();
            });
            let connect = tauri::async_runtime::spawn(async move {
                globals
                    .config
                    .connect_to_db_if_set(db_for_setup.clone())
                    .await
                    .map_err(|_| "Failed to Connect to provided DB");
                start(globals.clone()).await;
                let client: TorClient = globals.get_client().await.map_err(|e| e.to_string())?;
                Ok::<(), Box<dyn Error + Sync + Send + 'static>>(())
            });
            Ok(())
        })
        .invoke_handler(HANDLER)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}
