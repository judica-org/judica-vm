// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::view::{EmittedAppState, TradeType};
use super::{view::SyncError, *};
use crate::config::Globals;
use crate::tor::{GameHost, TorClient};
use crate::{Game, GameInitState, TriggerRerender};
use game_host_messages::{CreatedNewChain, FinishArgs, JoinCode};
use mine_with_friends_board::game::game_move::{GameMove, MintPowerPlant};
use mine_with_friends_board::game::UXUserInventory;
use mine_with_friends_board::nfts::instances::powerplant::PlantType;
use mine_with_friends_board::tokens::token_swap::{TradeError, TradeOutcome, TradingPairID};
use sapio_bitcoin::secp256k1::{All, Secp256k1};
use std::sync::Arc;
use tauri::async_runtime::Mutex;
use tauri::{generate_handler, Invoke};
use tokio::spawn;

pub const HANDLER: &(dyn Fn(Invoke) + Send + Sync) = &generate_handler![
    game_synchronizer,
    get_move_schema,
    get_materials_schema,
    get_purchase_schema,
    get_inventory_by_key,
    make_move_inner,
    switch_to_game,
    switch_to_db,
    set_signing_key,
    send_chat,
    make_new_chain,
    make_new_game,
    list_my_users,
    mint_power_plant_cost,
    super_mint,
    simulate_trade,
    set_game_host,
    finalize_game,
    disconnect_game,
    disconnect_game_host
];

#[tauri::command]
pub async fn simulate_trade(
    pair: TradingPairID,
    amounts: (u128, u128),
    trade: TradeType,
    signing_key: State<'_, SigningKeyInner>,
    s: GameState<'_>,
) -> Result<Result<TradeOutcome, TradeError>, SyncError> {
    view::simulate_trade(pair, amounts, trade, signing_key, s).await
}

#[tauri::command]
pub async fn game_synchronizer(
    window: Window,
    s: GameState<'_>,
    d: State<'_, Database>,
    game_host: State<'_, Arc<Mutex<Option<GameHost>>>>,
    signing_key: State<'_, SigningKeyInner>,
) -> Result<EmittedAppState, SyncError> {
    view::game_synchronizer_inner(window, s, d, game_host, signing_key).await
}

#[tauri::command]
pub(crate) async fn mint_power_plant_cost(
    scale: u64,
    location: (i64, i64),
    plant_type: PlantType,
    s: GameState<'_>,
    signing_key: State<'_, SigningKeyInner>,
) -> Result<Vec<(String, u128, u128)>, SyncError> {
    view::mint_power_plant_cost(scale, location, plant_type, s, signing_key).await
}
#[tauri::command]
pub(crate) async fn super_mint(
    scale: u64,
    location: (i64, i64),
    plant_type: PlantType,
    s: GameState<'_>,
    signing_key: State<'_, SigningKeyInner>,
    secp: State<'_, Arc<Secp256k1<All>>>,
    db: State<'_, Database>,
) -> Result<(), &'static str> {
    modify::make_move_inner_inner(
        secp.inner().clone(),
        db.inner().clone(),
        signing_key.inner().clone(),
        GameMove::SuperMintPowerPlant(MintPowerPlant {
            scale,
            location,
            plant_type,
        }),
    )
    .await
}

#[tauri::command]
pub(crate) async fn switch_to_db(
    db: State<'_, Database>,
    appName: String,
    prefix: Option<PathBuf>,
) -> Result<(), ()> {
    db.connect(&appName, prefix.clone()).await.map_err(|_| ())
}

#[tauri::command]
pub(crate) async fn set_signing_key(
    s: GameState<'_>,
    selected: Option<XOnlyPublicKey>,
    sk: State<'_, SigningKeyInner>,
) -> Result<(), ()> {
    modify::set_signing_key_inner(s, selected, sk).await
}

#[tauri::command]
pub(crate) fn get_move_schema() -> RootSchema {
    schema_for!(GameMove)
}

#[tauri::command]
pub(crate) fn get_purchase_schema() -> RootSchema {
    schema_for!(PurchaseNFT)
}

#[tauri::command]
pub(crate) fn get_materials_schema() -> RootSchema {
    schema_for!(Trade)
}

#[tauri::command]
pub(crate) async fn list_my_users(
    db: State<'_, Database>,
) -> Result<Vec<(XOnlyPublicKey, String)>, ()> {
    view::list_my_users_inner(db).await
}

#[tauri::command]
pub(crate) async fn make_new_chain(
    nickname: String,
    code: JoinCode,
    secp: State<'_, Arc<Secp256k1<All>>>,
    db: State<'_, Database>,
    globals: State<'_, Arc<Globals>>,
    game_host_state: State<'_, Arc<Mutex<Option<GameHost>>>>,
    game: GameState<'_>,
) -> Result<(), String> {
    modify::make_new_chain_inner(nickname, code, secp, db, globals, game_host_state, game).await
}

#[tauri::command]
pub(crate) async fn disconnect_game_host(
    game_host: State<'_, Arc<Mutex<Option<GameHost>>>>,
) -> Result<(), ()> {
    game_host.inner().lock().await.take();
    Ok(())
}
#[tauri::command]
pub(crate) async fn set_game_host(
    g: GameHost,
    game_host: State<'_, Arc<Mutex<Option<GameHost>>>>,
    globals: State<'_, Arc<Globals>>,
) -> Result<(), ()> {
    game_host.inner().lock().await.replace(g.clone());
    let client = globals.get_client().await.or(Err(()))?;
    // Courtesy Ping here which does DNS/Circuit Building to speed up subsequent
    // game joins
    spawn(async move { client.ping(&g).await });
    Ok(())
}
#[tauri::command]
pub(crate) async fn make_new_game(
    nickname: String,
    minutes: u16,
    secp: State<'_, Arc<Secp256k1<All>>>,
    db: State<'_, Database>,
    client: State<'_, Arc<Globals>>,
    game_host: State<'_, Arc<Mutex<Option<GameHost>>>>,
    game: GameState<'_>,
) -> Result<(), String> {
    modify::make_new_game(nickname, minutes, secp, db, client, game_host, game).await
}

#[tauri::command]
pub(crate) async fn finalize_game(
    args: FinishArgs,
    globals: State<'_, Arc<Globals>>,
    game_host_s: State<'_, Arc<Mutex<Option<GameHost>>>>,
) -> Result<CreatedNewChain, String> {
    let client = globals.get_client().await.map_err(|e| e.to_string())?;
    let game_host = game_host_s.lock().await.clone().ok_or("No Host")?;
    client
        .finish_setup(&game_host, args)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn send_chat(
    secp: State<'_, Arc<Secp256k1<All>>>,
    db: State<'_, Database>,
    sk: State<'_, SigningKeyInner>,
    chat: String,
) -> Result<(), &'static str> {
    modify::make_move_inner_inner(
        secp.inner().clone(),
        db.inner().clone(),
        sk.inner().clone(),
        GameMove::from(Chat(chat)),
    )
    .await
}

#[tauri::command]
pub(crate) async fn disconnect_game(
    sk: State<'_, SigningKeyInner>,
    game: GameState<'_>,
) -> Result<(), ()> {
    {
        let mut g = game.lock().await;
        *g = GameInitState::None;
    }
    {
        sk.lock().await.take();
    }
    Ok(())
}

#[tauri::command]
pub(crate) async fn switch_to_game(
    secp: State<'_, Arc<Secp256k1<All>>>,
    db: State<'_, Database>,
    sk: State<'_, SigningKeyInner>,
    game: GameState<'_>,
    key: XOnlyPublicKey,
) -> Result<(), ()> {
    modify::switch_to_game_inner(
        secp.inner().clone(),
        sk.inner().clone(),
        db.inner().clone(),
        game,
        key,
    )
    .await
}

#[tauri::command]
pub(crate) async fn make_move_inner(
    secp: State<'_, Arc<Secp256k1<All>>>,
    db: State<'_, Database>,
    sk: State<'_, SigningKeyInner>,
    nextMove: GameMove,
) -> Result<(), &'static str> {
    modify::make_move_inner_inner(
        secp.inner().clone(),
        db.inner().clone(),
        sk.inner().clone(),
        nextMove,
    )
    .await
}

#[tauri::command]
pub(crate) async fn get_inventory_by_key(
    game: GameState<'_>,
    user_key: String,
) -> Result<UXUserInventory, SyncError> {
    let res = view::get_user_inventory_by_key(game, user_key).await;
    res
}
