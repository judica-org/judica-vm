use std::sync::Arc;

use crate::{Database, Game, GameState, PrintOnDrop, SigningKeyInner};

use mine_with_friends_board::nfts::{
    instances::powerplant::PlantType, sale::UXForSaleList, NftPtr, UXPlantData,
};
use sapio_bitcoin::XOnlyPublicKey;

use serde::Serialize;
use tauri::{async_runtime::Mutex, State, Window};
use tokio::sync::futures::Notified;
use tracing::info;

pub(crate) async fn game_synchronizer_inner(
    window: Window,
    s: GameState<'_>,
    d: State<'_, Database>,
    signing_key: State<'_, SigningKeyInner>,
) -> Result<(), SyncError> {
    info!("Registering Window for State Updates");
    let _p = PrintOnDrop("Registration Canceled".into());
    loop {
        game_synchronizer_inner_loop(signing_key.inner(), s.inner(), d.inner(), &window).await?;
    }
}

#[derive(Serialize, Debug)]
pub enum SyncError {
    NoSigningKey,
    NoGame,
    KeyUnknownByGame,
    DatabaseError,
}

async fn game_synchronizer_inner_loop(
    signing_key: &Arc<Mutex<Option<XOnlyPublicKey>>>,
    s: &Arc<Mutex<Option<Game>>>,
    d: &Database,
    window: &Window,
) -> Result<(), SyncError> {
    // No Idea why the borrow checker likes this, but it seems to be the case
    // that because the notified needs to live inside the async state machine
    // hapily, giving a stable reference to it tricks the compiler into thinking
    // that the lifetime is 'static and we can successfully wait on it outside
    // the lock.
    let mut arc_cheat = None;
    let signing_key = *signing_key.lock().await;
    let signing_key = signing_key.ok_or(SyncError::NoSigningKey)?;
    let (gamestring, wait_on, key, chat_log, user_inventory) = {
        let game = s.lock().await;
        let game = game.as_ref().ok_or(SyncError::NoGame)?;
        let s = serde_json::to_string(&game.board).unwrap_or_else(|_| "null".into());
        arc_cheat = Some(game.should_notify.clone());
        let w: Option<Notified> = arc_cheat.as_ref().map(|x| x.notified());
        let chat_log = game.board.get_ux_chat_log();
        let user_inventory = game
            .board
            .get_ux_user_inventory(signing_key.to_string())
            .map_err(|()| SyncError::KeyUnknownByGame)?;
        (s, w, game.host_key, chat_log, user_inventory)
    };
    let (appName, prefix, list_of_chains, user_keys) = {
        let l = d.state.lock().await;
        if let Some(g) = l.as_ref() {
            let handle = g.db.get_handle().await;
            let v = handle
                .get_all_users()
                .map_err(|_| SyncError::DatabaseError)?;
            let keys: Vec<XOnlyPublicKey> = handle
                .get_keymap()
                .map_err(|_| SyncError::DatabaseError)?
                .into_keys()
                .collect();
            (g.name.clone(), g.prefix.clone(), v, keys)
        } else {
            ("".into(), None, vec![], vec![])
        }
    };
    // TODO: move these under a single held game lock
    // Attempt to get data to show prices
    let (raw_price_data, power_plants, listings) = {
        let mut game = s.lock().await;
        let game = game.as_mut().ok_or(SyncError::NoGame)?;
        let raw_price_data = game.board.get_ux_materials_prices().unwrap_or_default();
        let plants: Vec<(NftPtr, UXPlantData)> = game.board.get_ux_power_plant_data();

        let listings = game.board.get_ux_energy_market().unwrap_or(UXForSaleList {
            listings: Vec::new(),
        });
        (raw_price_data, plants, listings)
    };
    info!("Emitting State Updates");
    Ok(())
        .and_then(|()| window.emit("available-sequencers", list_of_chains))
        .and_then(|()| window.emit("chat-log", chat_log))
        .and_then(|()| window.emit("signing-key", signing_key))
        .and_then(|()| window.emit("host-key", key))
        .and_then(|()| window.emit("user-keys", user_keys))
        .and_then(|()| window.emit("db-connection", (appName, prefix)))
        .and_then(|()| window.emit("game-board", gamestring))
        .and_then(|()| window.emit("materials-price-data", raw_price_data))
        .and_then(|()| window.emit("power-plants", power_plants))
        .and_then(|()| window.emit("energy-exchange", listings.listings))
        .and_then(|()| window.emit("user-inventory", user_inventory))
        .expect("All window emits should succeed");
    if let Some(w) = wait_on {
        w.await;
    } else {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
    Ok(())
}

pub(crate) async fn list_my_users_inner(
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
        .map(|((_a, b), k)| (*k, b.clone()))
        .collect();
    Ok(ret)
}

/// returns qty BTC to purchase materials and mint plant of given type and size
pub(crate) async fn super_mint_power_plant_cost(
    scale: u64,
    location: (u64, u64),
    plant_type: PlantType,
    s: GameState<'_>,
    signing_key: State<'_, SigningKeyInner>,
) -> Result<u128, SyncError> {
    let cost = {
        let cost_detail =
            mint_power_plant_cost(scale, location, plant_type, s, signing_key).await?;
        let btc_costs: Vec<u128> = cost_detail.iter().map(|(_, _, btc)| *btc).collect();
        btc_costs.iter().sum()
    };

    Ok(cost)
}

// returns qty of each material necessary to mint plant of given type and size
pub(crate) async fn mint_power_plant_cost(
    scale: u64,
    location: (u64, u64),
    plant_type: PlantType,
    s: GameState<'_>,
    signing_key: State<'_, SigningKeyInner>,
) -> Result<Vec<(String, u128, u128)>, SyncError> {
    let signing_key: XOnlyPublicKey = signing_key
        .inner()
        .lock()
        .await
        .ok_or(SyncError::NoSigningKey)?;
    let mut game = s.inner().lock().await;
    let game = game.as_mut().ok_or(SyncError::NoGame)?;
    let current_prices = game
        .board
        .get_power_plant_cost(scale, location, plant_type, signing_key.to_string())
        .unwrap_or_default();

    Ok(current_prices)
}
