#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;

use mine_with_friends_board::{
    entity::EntityID,
    game::{
        game_move::{GameMove, Init},
        GameBoard,
    },
    Verified,
};
use schemars::{schema::RootSchema, schema_for};
use tauri::{
    async_runtime::{spawn, Mutex},
    State, Window,
};
use tokio::sync::Notify;

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

#[derive(Clone)]
struct Game(Arc<Mutex<GameBoard>>, Arc<Notify>);
fn main() {
    let game = Arc::new(Mutex::new(GameBoard::new()));
    let g = Game(game, Arc::new(Notify::new()));
    {
        let g = g.clone();
        spawn(async move {
            let mut game = g.0.lock().await;
            game.play(Verified::create(
                GameMove::Init(Init {}),
                1,
                "".into(),
                EntityID(0),
            ))
        });
    }
    tauri::Builder::default()
        .setup(|app| Ok(()))
        .manage(g.clone())
        .invoke_handler(tauri::generate_handler![game_synchronizer, get_move_schema])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
