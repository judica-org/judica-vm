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
        match game_synchronizer_inner_loop(signing_key.inner(), s.inner(), d.inner(), &window).await
        {
            Ok(()) => {}
            Err(e) => {
                tracing::debug!(?e, "SyncError");
                match &e {
                    SyncError::NoSigningKey | SyncError::NoGame => {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                    SyncError::KeyUnknownByGame | SyncError::DatabaseError => return Err(e),
                }
            }
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub enum SyncError {
    NoSigningKey,
    NoGame,
    KeyUnknownByGame,
    DatabaseError,
}
trait ResultFlipExt {
    type Output;
    fn flip(self) -> Self::Output;
}
impl<T, E1, E2> ResultFlipExt for Result<Result<T, E1>, E2> {
    type Output = Result<Result<T, E2>, E1>;

    fn flip(self) -> Self::Output {
        match self {
            Ok(Ok(v)) => Ok(Ok(v)),
            Err(e) => Ok(Err(e)),
            Ok(Err(v)) => Err(v),
        }
    }
}

async fn game_synchronizer_inner_loop(
    signing_key: &Arc<Mutex<Option<XOnlyPublicKey>>>,
    s: &Arc<Mutex<Option<Game>>>,
    d: &Database,
    window: &Window,
) -> Result<(), SyncError> {
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
    let signing_key = *signing_key.lock().await;
    let signing_key = signing_key.as_ref().ok_or(SyncError::NoSigningKey);

    Ok(())
        .and_then(|()| window.emit("db-connection", (appName, prefix)))
        .and_then({
            let signing_key = signing_key.clone();
            |()| {
                signing_key
                    .map(|key| emit(window, "signing-key", key))
                    .flip()?;
                Ok(())
            }
        })
        .and_then(|()| window.emit("available-sequencers", list_of_chains))
        .and_then(|()| window.emit("user-keys", user_keys))
        .expect("All window emits should succeed");

    // No Idea why the borrow checker likes this, but it seems to be the case
    // that because the notified needs to live inside the async state machine
    // hapily, giving a stable reference to it tricks the compiler into thinking
    // that the lifetime is 'static and we can successfully wait on it outside
    // the lock.
    let mut arc_cheat = None;
    let (
        gamestring,
        wait_on,
        key,
        chat_log,
        user_inventory,
        raw_price_data,
        power_plants,
        listings,
    ) = {
        let mut game = s.lock().await;
        let game = game.as_mut().ok_or(SyncError::NoGame)?;
        let s = serde_json::to_value(&game.board).unwrap_or_else(|e| {
            tracing::warn!(error=?e, "Failed to Serialized Game Board");
            serde_json::Value::Null
        });
        arc_cheat = Some(game.should_notify.clone());
        let w: Option<Notified> = arc_cheat.as_ref().map(|x| x.notified());
        let chat_log = game.board.get_ux_chat_log();
        let user_inventory = signing_key
            .as_ref()
            .map(|key| {
                game.board
                    .get_ux_user_inventory(key.to_string())
                    .map_err(|()| SyncError::KeyUnknownByGame)
            })
            .flip()?
            .map_err(|e| e.clone());
        // Attempt to get data to show prices
        let raw_price_data = game.board.get_ux_materials_prices().unwrap_or_default();
        let plants: Vec<(NftPtr, UXPlantData)> = game.board.get_ux_power_plant_data();

        let listings = game.board.get_ux_energy_market().unwrap_or(UXForSaleList {
            listings: Vec::new(),
        });

        (
            s,
            w,
            game.host_key,
            chat_log,
            user_inventory,
            raw_price_data,
            plants,
            listings,
        )
    };
    info!("Emitting State Updates");
    Ok(())
        .and_then(|()| emit(window, "chat-log", chat_log))
        .and_then(|()| emit(window, "host-key", key))
        .and_then(|()| emit(window, "game-board", gamestring))
        .and_then(|()| emit(window, "materials-price-data", raw_price_data))
        .and_then(|()| emit(window, "power-plants", power_plants))
        .and_then(|()| emit(window, "energy-exchange", listings.listings))
        .and_then(|()| {
            user_inventory
                .map(|inventory| emit(window, "user-inventory", inventory))
                .flip()?;
            Ok(())
        })
        .expect("All window emits should succeed");
    if let Some(w) = wait_on {
        w.await;
    } else {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
    Ok(())
}

fn emit<S>(window: &Window, event: &str, payload: S) -> Result<(), tauri::Error>
where
    S: Serialize + Clone + std::fmt::Debug,
{
    tracing::trace!(?payload, ?event, "Emitting: ");
    window.emit(event, payload)
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
    location: (i64, i64),
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

/// returns qty of each material necessary to mint plant of given type and size
pub(crate) async fn mint_power_plant_cost(
    scale: u64,
    location: (i64, i64),
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
