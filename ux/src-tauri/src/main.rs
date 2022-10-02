#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use attest_database::{connection::MsgDB, setup_db};
use commands::bindings::HANDLER;
use mine_with_friends_board::game::GameBoard;
use sapio_bitcoin::{secp256k1::Secp256k1, XOnlyPublicKey};
use serde::Deserialize;
use serde::Serialize;
use std::{error::Error, path::PathBuf, sync::Arc};
use tasks::GameServer;
use tauri::{async_runtime::Mutex, window, Manager, State};
use tokio::sync::Notify;
use tracing::info;
use tracing::warn;

mod commands;
mod tasks;

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

#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    db: Option<DBSelector>,
}
impl Config {
    async fn connect_to_db_if_set(&self, d: Database) -> Result<(), Box<dyn Error>> {
        if let Some(db) = &self.db {
            d.connect(&db.appname, db.prefix.clone()).await
        } else {
            Ok(())
        }
    }
}

#[derive(Serialize, Deserialize)]
struct DBSelector {
    appname: String,
    prefix: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    let game = GameStateInner::new(Mutex::new(None));
    let db = Database {
        state: Arc::new(Mutex::new(None)),
    };
    let db_for_setup = db.clone();
    let sk = SigningKeyInner::new(Mutex::new(None));
    tauri::Builder::default()
        .setup(move |app| {
            let app_handle = app.app_handle();
            app.listen_global("globe-click", move |e| {
                info!("globe-click payload {:?}:", e.payload().unwrap());
                app_handle
                    .emit_all("globe-location", e.payload().unwrap())
                    .unwrap();
            });
            let connect = tauri::async_runtime::spawn(async move {
                if let Ok(s) = std::env::var("MASTERMINE_CONFIG") {
                    let c: Config = serde_json::from_str(&s).map_err(|_| "Invalid Config")?;
                    c.connect_to_db_if_set(db_for_setup.clone()).await.map_err(|_| "Failed to Connect to provided DB");
                }
                Ok::<(), &'static str>(())
            });
            Ok(())
        })
        .manage(Arc::new(Secp256k1::new()))
        .manage(game)
        .manage(db)
        .manage(sk)
        .invoke_handler(HANDLER)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}
