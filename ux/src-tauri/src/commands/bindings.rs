use super::*;
#[tauri::command]
pub async fn game_synchronizer(
    window: Window,
    s: GameState<'_>,
    d: State<'_, Database>,
    signing_key: State<'_, SigningKeyInner>,
) -> Result<(), ()> {
    game_synchronizer_inner(window, s, d, signing_key).await
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
    set_signing_key_inner(s, selected, sk).await
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
    list_my_users_inner(db).await
}

#[tauri::command]
pub(crate) async fn make_new_chain(
    nickname: String,
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
) -> Result<String, String> {
    make_new_chain_inner(nickname, secp, db).await
}

#[tauri::command]
pub(crate) async fn send_chat(
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
    sk: State<'_, SigningKeyInner>,
    chat: String,
) -> Result<(), &'static str> {
    make_move_inner_inner(secp, db, sk, GameMove::from(Chat(chat)), EntityID(0)).await
}
#[tauri::command]
pub(crate) async fn switch_to_game(
    db: State<'_, Database>,
    game: GameState<'_>,
    key: XOnlyPublicKey,
) -> Result<(), ()> {
    switch_to_game_inner(db, game, key).await
}

#[tauri::command]
pub(crate) async fn make_move_inner(
    secp: State<'_, Secp256k1<All>>,
    db: State<'_, Database>,
    sk: State<'_, SigningKeyInner>,
    nextMove: GameMove,
    from: EntityID,
) -> Result<(), &'static str> {
    make_move_inner_inner(secp, db, sk, nextMove, from).await
}
