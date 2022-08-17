#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{sync::Arc, time::Duration};

use mine_with_friends_board::{
    entity::EntityID,
    game::{
        game_move::{GameMove, Init, RegisterUser},
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

#[tauri::command]
async fn make_move_inner(
    game: State<'_, Game>,
    nextMove: GameMove,
    from: EntityID,
) -> Result<(), ()> {
    {
        let mut game = game.0.lock().await;
        game.play_inner(nextMove, from);
    }
    game.1.notify_waiters();
    Ok(())
}

#[derive(Clone)]
struct Game(Arc<Mutex<GameBoard>>, Arc<Notify>);
fn main() {
    let game = Arc::new(Mutex::new(GameBoard::new()));
    let g = Game(game, Arc::new(Notify::new()));
    {
        let g = g.clone();
        spawn(async move {
            let root = {
                let mut game = g.0.lock().await;
                game.play_inner(GameMove::Init(Init {}), EntityID(0));
                game.root_user().unwrap()
            };
            g.1.notify_waiters();
            tokio::time::sleep(Duration::from_secs(3)).await;

            {
                let mut game = g.0.lock().await;
                game.play_inner(
                    GameMove::RegisterUser(RegisterUser {
                        user_id: "Alice".into(),
                    }),
                    root,
                );
                game.play_inner(
                    GameMove::RegisterUser(RegisterUser {
                        user_id: "Bob".into(),
                    }),
                    root,
                );
            }

            g.1.notify_waiters();
            tokio::time::sleep(Duration::from_secs(1)).await;
        });
    }

    tauri::Builder::default()
        .setup(|app| Ok(()))
        .manage(g.clone())
        .invoke_handler(tauri::generate_handler![
            game_synchronizer,
            get_move_schema,
            make_move_inner
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
