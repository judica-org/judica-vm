use std::sync::Arc;

use mine_with_friends_board::game::game_move::{GameMove, MintPowerPlant};
use mine_with_friends_board::nfts::instances::powerplant::PlantType;
use mine_with_friends_board::tokens::token_swap::{TradeError, TradeOutcome, TradingPairID};
use sapio_bitcoin::secp256k1::{All, Secp256k1};
use tauri::{generate_handler, Invoke};

use super::view::TradeType;
use super::{view::SyncError, *};
pub const HANDLER: &(dyn Fn(Invoke) + Send + Sync) = &generate_handler![
    game_synchronizer,
    get_move_schema,
    get_materials_schema,
    get_purchase_schema,
    make_move_inner,
    switch_to_game,
    switch_to_db,
    set_signing_key,
    send_chat,
    make_new_chain,
    list_my_users,
    mint_power_plant_cost,
    super_mint,
    simulate_trade
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
    signing_key: State<'_, SigningKeyInner>,
) -> Result<(), SyncError> {
    view::game_synchronizer_inner(window, s, d, signing_key).await
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
        EntityID(0),
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
    secp: State<'_, Arc<Secp256k1<All>>>,
    db: State<'_, Database>,
) -> Result<String, String> {
    modify::make_new_chain_inner(nickname, secp, db).await
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
        EntityID(0),
    )
    .await
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
    from: EntityID,
) -> Result<(), &'static str> {
    modify::make_move_inner_inner(
        secp.inner().clone(),
        db.inner().clone(),
        sk.inner().clone(),
        nextMove,
        from,
    )
    .await
}
