#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use attest_database::{connection::MsgDB, setup_db};
use mine_with_friends_board::game::GameBoard;
use sapio_bitcoin::{secp256k1::Secp256k1, XOnlyPublicKey};
use std::{error::Error, path::PathBuf, sync::Arc};
use tasks::GameServer;
use tauri::{async_runtime::Mutex, State};
use tokio::sync::Notify;
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
fn main() {
    tracing_subscriber::fmt::init();
    let game = GameStateInner::new(Mutex::new(None));
    let db = Database {
        state: Arc::new(Mutex::new(None)),
    };
    let sk = SigningKeyInner::new(Mutex::new(None));
    tauri::Builder::default()
        .setup(|_app| Ok(()))
        .manage(Secp256k1::new())
        .manage(game)
        .manage(db)
        .manage(sk)
        .invoke_handler(tauri::generate_handler![
            commands::bindings::game_synchronizer,
            commands::bindings::get_move_schema,
            commands::bindings::get_materials_schema,
            commands::bindings::get_purchase_schema,
            commands::bindings::make_move_inner,
            commands::bindings::switch_to_game,
            commands::bindings::switch_to_db,
            commands::bindings::set_signing_key,
            commands::bindings::send_chat,
            commands::bindings::make_new_chain,
            commands::bindings::list_my_users
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
