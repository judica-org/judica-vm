// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    tor::GameHost, Database, Game, GameInitState, GameState, GameStateInner, Pending, PrintOnDrop,
    SigningKeyInner, TriggerRerender,
};
use game_host_messages::{BroadcastByHost, Channelized, JoinCode};
use game_player_messages::ParticipantAction;
use mine_with_friends_board::{
    entity::EntityID,
    game::{GameBoard, GameSetup, UXUserInventory},
    nfts::{
        instances::powerplant::PlantType,
        sale::{UXForSaleList, UXNFTSale},
        NftPtr, UXPlantData,
    },
    tokens::token_swap::{TradeError, TradeOutcome, TradingPairID, UXMaterialsPriceData},
};
use sapio_bitcoin::XOnlyPublicKey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::VecDeque, ops::Deref, sync::Arc};
use std::{path::PathBuf, time::Duration};
use tauri::{
    async_runtime::{spawn_blocking, Mutex},
    State, Window,
};
use tokio::sync::futures::Notified;
use tracing::info;

pub(crate) async fn game_synchronizer_inner(
    window: Window,
    s: GameState<'_>,
    d: State<'_, Database>,
    g: State<'_, Arc<Mutex<Option<GameHost>>>>,
    signing_key: State<'_, SigningKeyInner>,
) -> Result<EmittedAppState, SyncError> {
    game_synchronizer_inner_loop(
        signing_key.inner(),
        s.inner(),
        g.inner(),
        d.inner(),
        &window,
    )
    .await
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

#[derive(Serialize, JsonSchema, Debug)]
pub struct EmittedAppState {
    db_connection: Option<(String, Option<PathBuf>)>,
    #[schemars(with = "String")]
    signing_key: Option<XOnlyPublicKey>,
    game_host_service: Option<GameHost>,
    #[schemars(with = "Vec<(String, GameSetup)>")]
    available_sequencers: Vec<(XOnlyPublicKey, GameSetup)>,
    #[schemars(with = "Vec<String>")]
    user_keys: Vec<XOnlyPublicKey>,
    #[serde(flatten)]
    game: Option<GameDependentEmitted>,
    super_handy_self_schema: Value,
    pending: Option<Pending>,
}

#[derive(Serialize, JsonSchema, Debug)]
struct GameDependentEmitted {
    chat_log: VecDeque<(u64, EntityID, String)>,
    #[schemars(with = "String")]
    host_key: XOnlyPublicKey,
    #[schemars(with = "GameBoard")]
    game_board: Value,
    materials_price_data: Vec<UXMaterialsPriceData>,
    power_plants: Vec<UXPlantData>,
    energy_exchange: Vec<UXNFTSale>,
    user_inventory: Option<UXUserInventory>,
}

async fn game_synchronizer_inner_loop(
    signing_key: &Arc<Mutex<Option<XOnlyPublicKey>>>,
    s: &Arc<Mutex<GameInitState>>,
    game_host: &Arc<Mutex<Option<GameHost>>>,
    d: &Database,
    window: &Window,
) -> Result<EmittedAppState, SyncError> {
    let (db_connection, available_sequencers, user_keys) = {
        let handle = if let Some(l) = d.state.lock().await.as_ref() {
            let name: String = l.name.clone();
            let path_buf: Option<PathBuf> = l.prefix.clone();
            let fut = l.db.get_handle_read();
            Some((fut.await, name, path_buf))
        } else {
            None
        };
        if let Some((handle, name, prefix)) = handle {
            spawn_blocking(move || {
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
                Ok::<_, SyncError>((Some((name, prefix)), sequencer_keys, user_keys))
            })
            .await
            .map_err(|_| SyncError::DatabaseError)??
        } else {
            (None, vec![], vec![])
        }
    };
    let signing_key_opt = *signing_key.lock().await;
    let signing_key = signing_key_opt.as_ref().ok_or(SyncError::NoSigningKey);
    let game_host_service = game_host.lock().await.clone();

    let pending = s.lock().await.pending_opt().cloned();
    let mut to_emit = EmittedAppState {
        db_connection,
        signing_key: signing_key_opt,
        game_host_service: game_host_service,
        super_handy_self_schema: serde_json::to_value(schemars::schema_for!(EmittedAppState))
            .unwrap(),
        available_sequencers,
        user_keys,
        pending,
        game: None,
    };
    // capture all errors from this section...
    async {
        let mut game = s.lock().await;
        let game = game.game_mut().ok_or(SyncError::NoGame)?;
        let game_value = serde_json::to_value(&game.board).unwrap_or_else(|e| {
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
            .map_err(|e| e.clone())
            .ok();
        // Attempt to get data to show prices
        let raw_price_data = game.board.get_ux_materials_prices();
        let plants: Vec<UXPlantData> = game.board.get_ux_power_plant_data();

        let listings = game.board.get_ux_energy_market().unwrap_or(UXForSaleList {
            listings: Vec::new(),
        });

        to_emit.game = Some(GameDependentEmitted {
            chat_log,
            host_key: game.host_key,
            game_board: game_value,
            materials_price_data: raw_price_data,
            power_plants: plants,
            energy_exchange: listings.listings,
            user_inventory,
        });
        Ok::<(), SyncError>(())
    }
    .await;
    Ok(to_emit)
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
    let handle = msgdb.get_handle_read().await;
    spawn_blocking(move || {
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
    })
    .await
    .or(Err(()))?
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
