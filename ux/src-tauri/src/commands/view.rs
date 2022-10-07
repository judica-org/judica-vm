use crate::{
    tor::GameHost, Database, Game, GameInitState, GameState, GameStateInner, Pending, PrintOnDrop,
    SigningKeyInner, TriggerRerender,
};
use game_host_messages::{BroadcastByHost, Channelized, JoinCode};
use game_player_messages::ParticipantAction;
use mine_with_friends_board::{
    game::GameSetup,
    nfts::{instances::powerplant::PlantType, sale::UXForSaleList, NftPtr, UXPlantData},
    tokens::token_swap::{TradeError, TradeOutcome, TradingPairID},
};
use sapio_bitcoin::XOnlyPublicKey;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{ops::Deref, sync::Arc};
use tauri::{async_runtime::Mutex, State, Window};
use tokio::sync::futures::Notified;
use tracing::info;

pub(crate) async fn game_synchronizer_inner(
    window: Window,
    s: GameState<'_>,
    d: State<'_, Database>,
    g: State<'_, Arc<Mutex<Option<GameHost>>>>,
    trigger: TriggerRerender,
    signing_key: State<'_, SigningKeyInner>,
) -> Result<(), SyncError> {
    info!("Registering Window for State Updates");
    let _p = PrintOnDrop("Registration Canceled".into());
    tokio::spawn({
        let s = s.inner().clone();
        let window = window.clone();
        let trigger = trigger.clone();
        async move {
            loop {
                let mut last = in_joining_mode(&s, &window, None).await?;
                if last.is_some() {
                    // wait till not pending
                    'in_mode: loop {
                        last = in_joining_mode(&s, &window, last).await?;
                        if last.is_some() {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        } else {
                            break 'in_mode;
                        }
                    }
                } else {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                };
            }
            Some(())
        }
    });
    loop {
        match game_synchronizer_inner_loop(
            signing_key.inner(),
            s.inner(),
            g.inner(),
            d.inner(),
            &window,
            trigger.clone(),
        )
        .await
        {
            Ok(()) => {}
            Err(e) => {
                tracing::debug!(?e, "SyncError");
                match &e {
                    SyncError::TradeError(_) => {}
                    SyncError::NoSigningKey | SyncError::NoGame => {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                    SyncError::KeyUnknownByGame | SyncError::DatabaseError => return Err(e),
                    SyncError::NoGameHost => {}
                    SyncError::HungUp => return Err(SyncError::HungUp),
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn in_joining_mode(
    s: &Arc<Mutex<GameInitState>>,
    window: &Window,
    should_emit_on_change: Option<JoinCode>,
) -> Option<Option<JoinCode>> {
    Some(if let GameInitState::Pending(p) = &*s.lock().await {
        if should_emit_on_change != Some(p.join_code) {
            emit(window, "game-init-admin", &p).ok()?;
        }
        Some(p.join_code)
    } else {
        None
    })
}

#[derive(Serialize, Debug, Clone)]
pub enum SyncError {
    HungUp,
    NoGameHost,
    NoSigningKey,
    NoGame,
    KeyUnknownByGame,
    DatabaseError,
    TradeError(TradeError),
}

impl From<TradeError> for SyncError {
    fn from(v: TradeError) -> Self {
        Self::TradeError(v)
    }
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
    s: &Arc<Mutex<GameInitState>>,
    game_host: &Arc<Mutex<Option<GameHost>>>,
    d: &Database,
    window: &Window,
    trigger: TriggerRerender,
) -> Result<(), SyncError> {
    let (db_connection, list_of_chains, user_keys) = {
        let l = d.state.lock().await;
        if let Some(g) = l.as_ref() {
            let handle = g.db.get_handle().await;
            let sequencer_keys: Vec<(XOnlyPublicKey, GameSetup)> = handle
                .get_all_genesis::<Channelized<BroadcastByHost>>()
                .map_err(|_| SyncError::DatabaseError)?
                .iter()
                .filter_map(|e| match &e.msg().data {
                    BroadcastByHost::GameSetup(g) => Some((e.header().key(), g.clone())),
                    BroadcastByHost::Sequence(_)
                    | BroadcastByHost::NewPeer(_)
                    | BroadcastByHost::Heartbeat => None,
                })
                .collect();
            let v = handle.get_keymap().map_err(|_| SyncError::DatabaseError)?;
            let user_keys: Vec<XOnlyPublicKey> = handle
                .get_all_genesis::<ParticipantAction>()
                .map_err(|_| SyncError::DatabaseError)?
                .iter()
                .map(|e| e.header().key())
                .filter(|k| v.contains_key(k))
                .collect();
            (
                Some((g.name.clone(), g.prefix.clone())),
                sequencer_keys,
                user_keys,
            )
        } else {
            (None, vec![], vec![])
        }
    };
    let signing_key = *signing_key.lock().await;
    let signing_key = signing_key.as_ref().ok_or(SyncError::NoSigningKey);
    let game_host_service = game_host.lock().await.clone();
    let game_host_service = game_host_service.as_ref().ok_or(SyncError::NoGameHost);

    info!("Emitting Basic Info Updates");
    Ok(())
        .and_then(|()| emit(window, "db-connection", db_connection))
        .and_then({
            let signing_key = signing_key.clone();
            |()| {
                signing_key
                    .map(|key| emit(window, "signing-key", key))
                    .flip()?;
                Ok(())
            }
        })
        .and_then({
            |()| {
                game_host_service
                    .map(|g| emit(window, "game-host-service", g))
                    .flip()?;
                Ok(())
            }
        })
        .and_then(|()| emit(window, "available-sequencers", list_of_chains))
        .and_then(|()| emit(window, "user-keys", user_keys))
        .expect("All window emits should succeed");

    let (gamestring, key, chat_log, user_inventory, raw_price_data, power_plants, listings) = {
        let mut game = s.lock().await;
        let game = game.game_mut().ok_or(SyncError::NoGame)?;
        let s = serde_json::to_value(&game.board).unwrap_or_else(|e| {
            tracing::warn!(error=?e, "Failed to Serialized Game Board");
            serde_json::Value::Null
        });
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
        let raw_price_data = game.board.get_ux_materials_prices();
        let plants: Vec<UXPlantData> = game.board.get_ux_power_plant_data();

        let listings = game.board.get_ux_energy_market().unwrap_or(UXForSaleList {
            listings: Vec::new(),
        });

        (
            s,
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
    let game = game.game_mut().ok_or(SyncError::NoGame)?;
    let current_prices =
        game.board
            .get_power_plant_cost(scale, location, plant_type, signing_key.to_string())?;

    Ok(current_prices)
}

#[derive(Serialize, Deserialize)]
pub enum TradeType {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
}
/// returns qty of each material necessary to mint plant of given type and size
pub(crate) async fn simulate_trade(
    pair: TradingPairID,
    amounts: (u128, u128),
    trade: TradeType,
    signing_key: State<'_, SigningKeyInner>,
    s: GameState<'_>,
) -> Result<Result<TradeOutcome, TradeError>, SyncError> {
    let sk = signing_key.lock().await;
    let sk = &sk.ok_or(SyncError::NoSigningKey)?;
    let sk = sk.to_string();
    let mut game = s.inner().lock().await;
    let game = game.game_mut().ok_or(SyncError::NoGame)?;
    let sender = game
        .board
        .get_user_id(&sk)
        .ok_or(SyncError::KeyUnknownByGame)?;
    Ok(match trade {
        TradeType::Buy => game
            .board
            .simulate_buy_trade(pair, amounts.0, amounts.1, sender),
        TradeType::Sell => game
            .board
            .simulate_sell_trade(pair, amounts.0, amounts.1, sender),
    })
}
