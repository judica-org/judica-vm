use mine_with_friends_board::game::game_move::GameMove;
use sapio_bitcoin::secp256k1::{All, Secp256k1};
use tauri::{generate_handler, Invoke};

use super::{*, view::SyncError};
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
    list_my_users
];
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
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
) -> Result<String, String> {
    modify::make_new_chain_inner(nickname, secp, db).await
}

#[tauri::command]
pub(crate) async fn send_chat(
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
    sk: State<'_, SigningKeyInner>,
    chat: String,
) -> Result<(), &'static str> {
    modify::make_move_inner_inner(secp, db, sk, GameMove::from(Chat(chat)), EntityID(0)).await
}
#[tauri::command]
pub(crate) async fn switch_to_game(
    db: State<'_, Database>,
    game: GameState<'_>,
    key: XOnlyPublicKey,
) -> Result<(), ()> {
    modify::switch_to_game_inner(db, game, key).await
}

#[tauri::command]
pub(crate) async fn make_move_inner(
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
    sk: State<'_, SigningKeyInner>,
    nextMove: GameMove,
    from: EntityID,
) -> Result<(), &'static str> {
    modify::make_move_inner_inner(secp, db, sk, nextMove, from).await
}
