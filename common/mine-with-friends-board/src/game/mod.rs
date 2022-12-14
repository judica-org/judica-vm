// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use self::game_move::Chat;
use self::game_move::GameMove;
use self::game_move::Heartbeat;
use self::game_move::ListNFTForSale;
use self::game_move::MintPowerPlant;
use self::game_move::PurchaseNFT;
use self::game_move::RemoveTokens;
use self::game_move::SendTokens;
use self::game_move::Trade;
use crate::callbacks::CallbackRegistry;
use crate::entity::EntityID;
use crate::entity::EntityIDAllocator;
use crate::nfts::instances::powerplant::events::PowerPlantEvent;
use crate::nfts::instances::powerplant::PlantType;
use crate::nfts::instances::powerplant::PowerPlant;
use crate::nfts::instances::powerplant::PowerPlantPrices;
use crate::nfts::instances::powerplant::PowerPlantProducer;
use crate::nfts::sale::NFTSaleRegistry;
use crate::nfts::sale::UXForSaleList;
use crate::nfts::sale::UXNFTSale;
use crate::nfts::BaseNFT;
use crate::nfts::NFTRegistry;
use crate::nfts::NftPtr;
use crate::nfts::UXNFTRegistry;
use crate::nfts::UXPlantData;
use crate::sanitize::Sanitizable;
use crate::tokens;
use crate::tokens::instances::asics::ASICProducer;
use crate::tokens::instances::asics::HashBoardData;
use crate::tokens::instances::concrete::ConcreteMiller;
use crate::tokens::instances::silicon::Silicon;
use crate::tokens::instances::silicon::SiliconRefinery;
use crate::tokens::instances::steel::Steel;
use crate::tokens::instances::steel::SteelSmelter;
use crate::tokens::token_swap;
use crate::tokens::token_swap::ConstantFunctionMarketMaker;
use crate::tokens::token_swap::ConstantFunctionMarketMakerPair;
use crate::tokens::token_swap::TradeError;
use crate::tokens::token_swap::TradeOutcome;
use crate::tokens::token_swap::TradingPairID;
use crate::tokens::token_swap::UXMaterialsPriceData;
use crate::MoveEnvelope;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::cmp::max;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::Duration;
use tokens::TokenBase;
use tokens::TokenPointer;
use tokens::TokenRegistry;
use tracing::info;
use tracing::trace;

#[derive(Debug, Serialize, Clone, JsonSchema)]
#[serde(tag = "event_type")]
pub enum LogEvent {
    GameMove(GameMove),
    MoveRejectReason(MoveRejectReason),
    Other(serde_json::Value),
}

#[derive(Serialize, Clone, Debug, JsonSchema)]
pub struct UXUserInventory {
    user_power_plants: BTreeMap<NftPtr, UXPlantData>,
    user_token_balances: Vec<(String, u128)>,
}
#[derive(Serialize, JsonSchema, Debug)]
pub struct UserData {
    pub key: String,
}

#[derive(Serialize, Clone, JsonSchema, Debug)]
pub struct Tick {
    first_time: u64,
    elapsed: u64,
}
/// GameBoard holds the entire state of the game.
#[derive(Serialize, JsonSchema, Debug)]
pub struct GameBoard {
    pub(crate) tokens: tokens::TokenRegistry,
    pub(crate) swap: token_swap::ConstantFunctionMarketMaker,
    /// Make this a vote over the map of users to current vote and let the turn count be dynamic
    pub(crate) turn_count: u64,
    pub(crate) alloc: EntityIDAllocator,
    pub(crate) users: BTreeMap<EntityID, UserData>,
    pub(crate) users_by_key: BTreeMap<String, EntityID>,
    pub(crate) nfts: NFTRegistry,
    pub(crate) nft_sales: NFTSaleRegistry,
    pub(crate) player_move_sequence: BTreeMap<EntityID, u64>,
    /// If init = true, must be Some
    pub(crate) bitcoin_token_id: TokenPointer,
    /// If init = true, must be Some
    pub(crate) real_sats_token_id: TokenPointer,
    /// If init = true, must be Some
    pub(crate) steel_token_id: TokenPointer,
    /// If init = true, must be Some
    pub(crate) silicon_token_id: TokenPointer,
    /// If init = true, must be Some
    pub(crate) concrete_token_id: TokenPointer,
    pub(crate) asic_token_id: TokenPointer,
    pub(crate) root_user: EntityID,
    pub(crate) callbacks: CallbackRegistry,
    pub(crate) elapsed_time: u64,
    pub(crate) finish_time: u64,
    pub(crate) mining_subsidy: u128,
    pub ticks: BTreeMap<EntityID, Tick>,
    pub chat: VecDeque<(u64, EntityID, String)>,
    pub nicks: BTreeMap<EntityID, String>,
    pub chat_counter: u64,
    pub event_log: VecDeque<(u64, EntityID, LogEvent)>,
    pub event_log_counter: u64,
    pub(crate) plant_prices: PowerPlantPrices,
}

pub struct CallContext {
    pub sender: EntityID,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct GameSetup {
    // TODO: Make Set to guarantee Unique...
    pub players: Vec<String>,
    pub start_amount: u64,
    // TODO: maybe remove no_finish_time default, but helps with existing chains...
    #[serde(default = "no_finish_time")]
    pub finish_time: u64,
}
fn no_finish_time() -> u64 {
    // otherwise breaks json
    9_007_199_254_740_991u64
}
impl GameSetup {
    fn setup_game(&self, g: &mut GameBoard) {
        g.finish_time = self.finish_time;
        let mut p = self.players.clone();
        p.sort();
        p.dedup();
        for player in p {
            let id = g.alloc();
            g.users.insert(
                id,
                UserData {
                    key: player.clone(),
                },
            );
            g.users_by_key.insert(player.clone(), id);
            g.tokens[g.bitcoin_token_id].transaction();
            g.tokens[g.real_sats_token_id].transaction();
            {
                g.tokens[g.real_sats_token_id].mint(&id, self.start_amount as u128);
                g.tokens[g.bitcoin_token_id].mint(&id, self.start_amount as u128);
            }
            g.tokens[g.bitcoin_token_id].end_transaction();
            g.tokens[g.real_sats_token_id].end_transaction();
        }
    }
}

#[derive(Serialize, Clone, Debug, JsonSchema)]
pub enum FinishReason {
    TimeExpired,
    DominatingPlayer(EntityID),
}

#[derive(Serialize, Clone, Debug, JsonSchema)]
pub enum MoveRejectReason {
    NoSuchUser,
    GameIsFinished(FinishReason),
    MoveSanitizationError(<GameMove as Sanitizable>::Error),
    TradeRejected(TradeError),
}

impl From<TradeError> for MoveRejectReason {
    fn from(v: TradeError) -> Self {
        Self::TradeRejected(v)
    }
}

impl From<FinishReason> for MoveRejectReason {
    fn from(v: FinishReason) -> Self {
        Self::GameIsFinished(v)
    }
}

impl From<<GameMove as Sanitizable>::Error> for MoveRejectReason {
    fn from(v: <GameMove as Sanitizable>::Error) -> Self {
        Self::MoveSanitizationError(v)
    }
}
#[derive(Serialize, Clone, Debug)]
pub enum CloseError {
    GameNotFinished,
}
impl GameBoard {
    /// Creates a new GameBoard
    pub fn new(setup: &GameSetup) -> GameBoard {
        let mut alloc = EntityIDAllocator::new();

        let btc = Box::new(TokenBase::new_from_alloc(&mut alloc, "Virtual Sats".into()));
        let real_sats = Box::new(TokenBase::new_from_alloc(
            &mut alloc,
            "Real World Sats".into(),
        ));
        let concrete = Box::new(TokenBase::new_from_alloc(
            &mut alloc,
            "Concrete (1mt)".into(),
        ));
        let asic = Box::new(TokenBase::new_from_alloc(&mut alloc, "ASIC Gen 1".into()));
        let steel = Box::new(TokenBase::new_from_alloc(&mut alloc, "Steel (1mt)".into()));
        let silicon = Box::new(TokenBase::new_from_alloc(
            &mut alloc,
            "Silicon (10kg)".into(),
        ));
        let mut tokens = TokenRegistry::default();
        let bitcoin_token_id = tokens.new_token(btc);
        let real_sats_token_id = tokens.new_token(real_sats);
        let concrete_token_id = tokens.new_token(concrete);
        let steel_token_id = tokens.new_token(steel);
        let silicon_token_id = tokens.new_token(silicon);
        let asic_token_id = tokens.new_token(asic);
        tokens.hashboards.insert(
            asic_token_id,
            HashBoardData {
                hash_per_watt: 3 * 10e10 as u128,
                reliability: 100,
            },
        );
        tokens.steel.insert(
            steel_token_id,
            Steel {
                variety: tokens::instances::steel::SteelVariety::Structural,
                weight_in_kg: 1000,
            },
        );
        tokens
            .silicon
            .insert(silicon_token_id, Silicon { weight_in_kg: 10 });

        let root_user = alloc.make();
        let mut plant_prices = HashMap::new();
        plant_prices.insert(
            PlantType::Solar,
            Vec::from([
                (steel_token_id, 1),
                (silicon_token_id, 8),
                (concrete_token_id, 1),
            ]),
        );
        plant_prices.insert(
            PlantType::Hydro,
            Vec::from([
                (steel_token_id, 3),
                (silicon_token_id, 1),
                (concrete_token_id, 6),
            ]),
        );
        plant_prices.insert(
            PlantType::Flare,
            Vec::from([
                (steel_token_id, 4),
                (silicon_token_id, 2),
                (concrete_token_id, 4),
            ]),
        );

        let mut g = GameBoard {
            tokens,
            swap: Default::default(),
            turn_count: 0,
            bitcoin_token_id,
            real_sats_token_id,
            steel_token_id,
            silicon_token_id,
            concrete_token_id,
            asic_token_id,
            root_user,
            alloc,
            users: Default::default(),
            users_by_key: Default::default(),
            nfts: Default::default(),
            nft_sales: Default::default(),
            player_move_sequence: Default::default(),
            callbacks: Default::default(),
            elapsed_time: 0,
            finish_time: 0,
            mining_subsidy: 100_000_000 * 50,
            ticks: Default::default(),
            chat: VecDeque::with_capacity(1000),
            chat_counter: 0,
            event_log: VecDeque::with_capacity(1000),
            event_log_counter: 0,
            plant_prices,
            nicks: Default::default(),
        };
        setup.setup_game(&mut g);
        g.post_init();
        g
    }

    fn post_init(&mut self) {
        {
            let make = |id, g: &mut GameBoard| {
                let btc = g.bitcoin_token_id;
                ConstantFunctionMarketMakerPair::ensure(
                    g,
                    TradingPairID {
                        asset_a: btc,
                        asset_b: id,
                    },
                )
            };
            make(self.steel_token_id, self);
            make(self.silicon_token_id, self);
            make(self.concrete_token_id, self);
            make(self.asic_token_id, self);
        }
        self.tokens[self.bitcoin_token_id].transaction();
        self.tokens[self.real_sats_token_id].transaction();
        self.tokens[self.bitcoin_token_id].mint(&self.root_user, 10_000_000_000);
        self.tokens[self.real_sats_token_id].mint(&self.root_user, 30000);
        self.tokens[self.bitcoin_token_id].end_transaction();
        self.tokens[self.real_sats_token_id].end_transaction();
        //
        let id = self.alloc();
        self.callbacks.schedule(Box::new(PowerPlantEvent {
            // Next Move
            time: 0,
            period: 11_003, // 11 seconds,
        }));
        self.callbacks.schedule(Box::new(ASICProducer {
            id,
            total_units: 100_000,
            base_price: 100_000,
            price_asset: self.bitcoin_token_id,
            hash_asset: *self.tokens.hashboards.iter().next().unwrap().0,
            adjusts_every: 10_007, // 10 seconds -- prime rounded for chaos
            elapsed_time: 0,
            first: true,
        }));
        let steel_id = self.alloc();
        self.callbacks.schedule(Box::new(SteelSmelter {
            id: steel_id,
            total_units: 100_000,
            base_price: 100_000,
            price_asset: self.bitcoin_token_id,
            hash_asset: self.steel_token_id,
            adjusts_every: 5_003, // 5 seconds
            elapsed_time: 0,
            first: true,
        }));
        let silicon_id = self.alloc();
        self.callbacks.schedule(Box::new(SiliconRefinery {
            id: silicon_id,
            total_units: 100_000,
            base_price: 100_000,
            price_asset: self.bitcoin_token_id,
            hash_asset: self.silicon_token_id,
            adjusts_every: 25_013, // 25 seconds
            elapsed_time: 0,
            first: true,
        }));
        let concrete_id = self.alloc();
        self.callbacks.schedule(Box::new(ConcreteMiller {
            id: concrete_id,
            total_units: 100_000,
            base_price: 100_000,
            price_asset: self.bitcoin_token_id,
            hash_asset: self.concrete_token_id,
            adjusts_every: 14_009, // 14 seconds
            elapsed_time: 0,
            first: true,
        }));

        #[cfg(not(test))]
        {
            let start_locations: Vec<((i64, i64), PlantType)> = vec![
                ((38075660, -120030170), PlantType::Flare),
                ((-2346660, -76223050), PlantType::Hydro),
                ((-5203060, -66345370), PlantType::Solar),
                ((66596730, 31545250), PlantType::Flare),
                ((29521143, 26620126), PlantType::Hydro),
                ((58162163, 39188486), PlantType::Solar),
                ((28599246, 116180671), PlantType::Flare),
                ((-29625744, 138416999), PlantType::Hydro),
                ((-12141092, 35233406), PlantType::Solar),
                ((21968789, -12535149), PlantType::Flare),
            ];

            let mut p: Vec<EntityID> = self
                .users_by_key
                .iter()
                .map(|(_, p_id)| EntityID(p_id.0))
                .collect();
            p.sort();
            p.dedup(); // necessary?

            for (i, player) in p.iter().enumerate() {
                // base nft
                let base_nft = BaseNFT {
                    owner: *player,
                    nft_id: self.alloc(),
                    transfer_count: 0,
                };
                let plant_ptr = self.nfts.add(Box::new(base_nft));
                // pick random plant type
                let random_plant_type = start_locations[i].1;
                // pick random location
                let coordinates: (i64, i64) = start_locations[i].0;

                let new_plant =
                    PowerPlant::new(self, plant_ptr, random_plant_type, coordinates, 1 as u64);
                // add to plant register, need to return Plant?
                let _ = self.nfts.power_plants.insert(plant_ptr, new_plant);
                self.tokens[self.asic_token_id].mint(&plant_ptr.inner(), 1);
            }
        }
    }
    /// Creates a new EntityID
    pub fn alloc(&mut self) -> EntityID {
        self.alloc.make()
    }
    pub fn root_user(&self) -> EntityID {
        self.root_user
    }
    pub fn user_data(&self) -> &BTreeMap<EntityID, UserData> {
        todo!()
    }
    pub fn user_shares(&self) -> (u128, BTreeMap<EntityID, u128>) {
        todo!()
    }
    pub fn user_hashrates(&self) -> (u128, BTreeMap<EntityID, u128>) {
        todo!()
    }
    pub fn current_time(&self) -> u64 {
        self.elapsed_time
    }
    /// Check if a given user is the root_user
    pub fn user_is_admin(&self, user: EntityID) -> bool {
        user == self.root_user
    }

    ///Get the distributions of rewards mapped by player key
    // TODO: Imbue with Oracle Key somewhere?
    pub fn get_close_distribution(
        &self,
        bounty: u64,
        host_key: String,
    ) -> Result<Vec<(String, u64)>, CloseError> {
        let mut v = vec![];
        match self.game_is_finished().ok_or(CloseError::GameNotFinished)? {
            FinishReason::TimeExpired => {
                let balances = self
                    .users_by_key
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            self.tokens[self.bitcoin_token_id].balance_check(v),
                        )
                    })
                    .collect::<Vec<_>>();
                let total = balances.iter().map(|(_, v)| v).sum::<u128>();
                let mut rewards: Vec<(String, u64)> = if total == 0 {
                    let players = balances.len() as u64;
                    balances
                        .into_iter()
                        .map(|(k, _v)| {
                            #[allow(clippy::integer_division)]
                            (k, bounty / players)
                        })
                        .collect()
                } else {
                    balances
                        .into_iter()
                        .map(|(k, v)| {
                            #[allow(clippy::integer_division)]
                            (k, ((v * bounty as u128) / total) as u64)
                        })
                        .collect()
                };
                let excess = bounty - rewards.iter().map(|(_, v)| v).sum::<u64>();
                if let Some(m) = rewards.first_mut() {
                    m.1 += excess;
                }
                Ok(rewards)
            }
            FinishReason::DominatingPlayer(id) => {
                let key = self.users[&id].key.clone();
                // 75%
                #[allow(clippy::integer_division)]
                let twentyfivepercent = bounty / 4;
                v.push((key, (bounty - twentyfivepercent)));
                // 25%
                v.push((host_key, twentyfivepercent));
                Ok(v)
            }
        }
    }
    /// Processes a GameMove against the board after verifying it's integrity
    /// and sanitizing it.
    pub fn play(
        &mut self,
        MoveEnvelope {
            d,
            sequence,
            time_millis,
        }: MoveEnvelope,
        signed_by: String,
    ) -> Result<(), MoveRejectReason> {
        let from = *self
            .users_by_key
            .get(&signed_by)
            .ok_or(MoveRejectReason::NoSuchUser)?;

        if let Some(finish_reason) = self.game_is_finished() {
            self.add_to_event_log(
                from,
                LogEvent::MoveRejectReason(MoveRejectReason::GameIsFinished(finish_reason.clone())),
            );
            return Err(MoveRejectReason::GameIsFinished(finish_reason));
        }

        info!(key = signed_by, ?from, "Got Move {} From Player", sequence);
        // TODO: check that sequence is the next sequence for that particular user
        let current_move = self.player_move_sequence.entry(from).or_default();
        if (*current_move + 1) != sequence && !(*current_move == 0 && sequence == 0) {
            return Ok(());
        } else {
            *current_move = sequence;
        }
        let mv = d.sanitize(self)?;
        self.update_current_time((from, time_millis));
        self.process_ticks();

        // TODO: verify the key/sig/d combo (or it happens during deserialization of Verified)
        trace!(?mv, "Attempting Inner Move");
        self.add_to_event_log(from, LogEvent::GameMove(mv.clone()));
        match self.play_inner(mv, from) {
            Ok(_) => info!("Move Successfully Made"),
            Err(e) => {
                self.add_to_event_log(from, LogEvent::MoveRejectReason(e.clone()));
                return Err(e);
            }
        }
        Ok(())
    }

    pub fn process_ticks(&mut self) {
        CallbackRegistry::run(self);
    }

    fn update_current_time(&mut self, (from, time): (EntityID, u64)) {
        trace!(?from, time, "updating time for player");
        let tick = self.ticks.entry(from).or_insert(Tick {
            first_time: time,
            elapsed: 0,
        });
        tick.elapsed = max(
            tick.elapsed,
            time.checked_sub(tick.first_time).unwrap_or_default(),
        );
        trace!(elapsed = ?Duration::from_millis(tick.elapsed), player=?from);
        let mut elapsed: Vec<u64> = self.ticks.values().map(|t| t.elapsed).collect();
        elapsed.sort_unstable();
        trace!(?elapsed, "elapsed times");
        // todo: maybe ensure monotonic?
        self.elapsed_time = self.elapsed_time.max(if elapsed.len() % 2 == 0 {
            (elapsed.get(elapsed.len() / 2).cloned().unwrap_or_default()
                + elapsed
                    .get((elapsed.len() / 2) - 1)
                    .cloned()
                    .unwrap_or_default())
                / 2
        } else {
            elapsed.get(elapsed.len() / 2).cloned().unwrap_or_default()
        });
        trace!(elapsed = ?Duration::from_millis(self.elapsed_time), player=?from, "New Median");
    }

    pub fn game_is_finished(&self) -> Option<FinishReason> {
        if self.elapsed_time >= self.finish_time {
            trace!(self.elapsed_time, self.finish_time, "Game Time Expired");
            Some(FinishReason::TimeExpired)
        } else if self.elapsed_time >= (3 * self.finish_time / 4) {
            // After 75 % of the game is finished...
            self.get_user_hashrate_share()
                .iter()
                .find_map(|(k, v)| if v.0 * 2 >= v.1 { Some(*k) } else { None })
                .map(FinishReason::DominatingPlayer)
        } else {
            None
        }
    }
    /// Processes a GameMove without any sanitization
    pub fn play_inner(&mut self, d: GameMove, from: EntityID) -> Result<(), MoveRejectReason> {
        // TODO: verify the key/sig/d combo (or it happens during deserialization of Verified)
        let context = CallContext { sender: from };
        match d {
            GameMove::Heartbeat(Heartbeat()) => {}
            GameMove::Trade(Trade {
                pair,
                amount_a,
                amount_b,
                sell,
                cap,
            }) => {
                if sell {
                    ConstantFunctionMarketMaker::do_sell_trade(
                        self, pair, amount_a, amount_b, cap, false, &context,
                    )?;
                } else {
                    ConstantFunctionMarketMaker::do_buy_trade(
                        self, pair, amount_a, amount_b, cap, false, &context,
                    )?;
                }
            }
            GameMove::MintPowerPlant(MintPowerPlant {
                scale,
                location,
                plant_type,
            }) => {
                PowerPlantProducer::mint_power_plant(
                    self,
                    scale,
                    location,
                    plant_type,
                    context.sender,
                )?;
            }
            GameMove::SuperMintPowerPlant(MintPowerPlant {
                scale,
                location,
                plant_type,
            }) => {
                PowerPlantProducer::super_mint(self, scale, location, plant_type, context.sender)?;
            }
            GameMove::PurchaseNFT(PurchaseNFT {
                nft_id,
                limit_price,
                currency,
            }) => self.nft_sales.purchase(
                &context,
                nft_id,
                &mut self.tokens,
                &mut self.nfts,
                limit_price,
                currency,
            ),
            GameMove::ListNFTForSale(ListNFTForSale {
                nft_id,
                price,
                currency,
            }) => self
                .nft_sales
                .list_nft(&context, nft_id, price, currency, &self.nfts),
            GameMove::SendTokens(SendTokens {
                to,
                amount,
                currency,
            }) => {
                self.tokens[currency].transaction();
                let _ = self.tokens[currency].transfer(&from, &to, amount);
                self.tokens[currency].end_transaction();
            }
            GameMove::RemoveTokens(RemoveTokens {
                nft_id,
                amount,
                currency,
            }) => {
                let shipping_time = 1;
                let owner = &self.nfts.nfts[&nft_id].owner();
                if owner.eq(&from) {
                    let plant = &self.nfts.power_plants[&nft_id];
                    plant
                        .to_owned()
                        .ship_hashrate(currency, amount, shipping_time, self);
                } else {
                    info!("Remove Tokens: NFT owner mismatch");
                }
            }
            GameMove::Chat(Chat(mut s)) => {
                if s.starts_with("/nick") && s.is_ascii() && s.len() < 32 {
                    let nick = s.split_at(s.find(' ').unwrap_or(s.len()));
                    self.nicks.insert(from, nick.1.to_owned());
                    s = format!("{} is now known as {}", String::from(from), nick.1);
                }
                self.chat_counter += 1;
                // only log the last 1000 messages
                // TODO: Configurable? Ignorable?
                if self.chat.len() >= 1000 {
                    self.chat.pop_front();
                }
                self.chat.push_back((self.chat_counter, from, s));
            }
        }
        Ok(())
    }

    pub fn get_ux_chat_log(&self) -> VecDeque<(u64, EntityID, String)> {
        self.chat.clone()
    }

    pub fn get_ux_event_log(&self) -> VecDeque<(u64, EntityID, LogEvent)> {
        self.event_log.clone()
    }

    pub(crate) fn add_to_event_log(&mut self, from: EntityID, e: LogEvent) {
        self.event_log_counter += 1;
        if self.event_log.len() >= 1000 {
            self.event_log.pop_front();
        }
        self.event_log.push_back((self.event_log_counter, from, e))
    }

    pub fn get_ux_materials_prices(&mut self) -> Vec<UXMaterialsPriceData> {
        let mut res = vec![];
        for id in self.tokens.tokens.keys().cloned().collect::<Vec<_>>() {
            let ptr = self.tokens.tokens[&id].ptr();
            let nick = self.tokens.tokens[&id]
                .nickname()
                .unwrap_or_else(|| "Unknown Token".into());

            let mut trading_pair = TradingPairID {
                asset_a: ptr,
                asset_b: self.bitcoin_token_id,
            };
            trading_pair.normalize();
            // N.B. Pairs may not show up on first pass if not formerly ensured
            if !ConstantFunctionMarketMakerPair::has_market(self, trading_pair) {
                continue;
            }
            let (mkt_qty_a, mkt_qty_b) =
                ConstantFunctionMarketMaker::get_pair_price_data(self, trading_pair);
            res.push(UXMaterialsPriceData {
                asset_a: self.tokens[trading_pair.asset_a]
                    .nickname()
                    .unwrap_or_else(|| format!("Unknown Token: {:?}", trading_pair.asset_a)),
                asset_b: self.tokens[trading_pair.asset_b]
                    .nickname()
                    .unwrap_or_else(|| format!("Unknown Token: {:?}", trading_pair.asset_b)),
                mkt_qty_a,
                mkt_qty_b,
                trading_pair,
                display_asset: nick,
            })
        }
        res
    }

    // where does miner status come from
    pub fn get_ux_power_plant_data(&self) -> Vec<UXPlantData> {
        let plants = &self.nfts.power_plants.clone();
        let power_plant_data = plants
            .iter()
            .map(|(pointer, power_plant)| {
                let for_sale = self.nft_sales.nfts.get(pointer).is_some();
                let nft = &self.nfts[*pointer];
                let owner = nft.owner();
                let nft_entity_id = nft.id();
                let asic_token_id = self.asic_token_id;
                let miners = self.tokens[asic_token_id].balance_check(&nft_entity_id);

                UXPlantData {
                    id: *pointer,
                    coordinates: power_plant.coordinates,
                    for_sale,
                    miners,
                    owner,
                    plant_type: power_plant.plant_type,
                    watts: power_plant.watts,
                    hashrate: power_plant.compute_hashrate(self),
                }
            })
            .collect();
        power_plant_data
    }

    pub fn get_user_power_plants(&self, user_id: EntityID) -> Result<UXNFTRegistry, ()> {
        let mut power_plant_data = BTreeMap::new();
        let mut power_plant_vec = self.get_ux_power_plant_data();
        // should use something other than drain_filter?
        power_plant_vec.retain(|plant| plant.owner.eq(&user_id));
        power_plant_vec.iter().for_each(|plant| {
            power_plant_data.insert(plant.id, plant.clone());
        });
        // return shape?
        Ok(UXNFTRegistry { power_plant_data })
    }

    pub fn get_ux_user_inventory(&self, user_key: String) -> Result<UXUserInventory, ()> {
        let user_id = self.users_by_key.get(&user_key).unwrap().to_owned();
        let user_power_plants = self
            .get_user_power_plants(user_id)
            .unwrap()
            .power_plant_data;
        let user_token_balances = {
            let mut balances = Vec::new();
            for token in self.tokens.tokens.values() {
                let balance = token.balance_check(&user_id);
                let nickname = token.nickname().unwrap();
                balances.push((nickname, balance))
            }
            balances
        };
        Ok(UXUserInventory {
            user_power_plants,
            user_token_balances,
        })
    }

    pub fn get_ux_energy_market(&self) -> Result<UXForSaleList, ()> {
        let mut listings = Vec::new();
        self.nft_sales.nfts.iter().for_each(|(pointer, listing)| {
            listings.push(UXNFTSale {
                nft_id: *pointer,
                price: listing.price,
                currency: listing.currency,
                seller: listing.seller,
                transfer_count: listing.transfer_count,
            });
        });
        Ok(UXForSaleList { listings })
    }
    pub fn get_user_hashrate_share(&self) -> BTreeMap<EntityID, (u128, u128)> {
        let denominator = 100000u128;
        let reg = &self.nfts;
        let mut res = BTreeMap::new();
        let mut total = 0u128;
        // accumulation step
        for (ptr, plant) in reg.power_plants.iter() {
            let rate = plant.compute_hashrate(self);
            let player = reg.nfts[ptr].owner();
            match res.get_mut(&player) {
                None => {
                    res.insert(player, (rate * denominator, denominator));
                }
                Some(v) => {
                    v.0 += rate * denominator;
                }
            }
            total += rate;
        }
        // normalization step
        if total > 0 {
            res.iter_mut().for_each(|(_, v)| v.0 /= total);
        }
        res
    }

    pub fn simulate_buy_trade(
        &mut self,
        pair: TradingPairID,
        amount_a: u128,
        amount_b: u128,
        sender: EntityID,
    ) -> Result<TradeOutcome, TradeError> {
        match ConstantFunctionMarketMaker::do_buy_trade(
            self,
            pair,
            amount_a,
            amount_b,
            None,
            true,
            &CallContext { sender },
        ) {
            Ok(outcome) => Ok(outcome),
            Err(e) => Err(e),
        }
    }
    pub fn simulate_sell_trade(
        &mut self,
        pair: TradingPairID,
        amount_a: u128,
        amount_b: u128,
        sender: EntityID,
    ) -> Result<TradeOutcome, TradeError> {
        match ConstantFunctionMarketMaker::do_sell_trade(
            self,
            pair,
            amount_a,
            amount_b,
            None,
            true,
            &CallContext { sender },
        ) {
            Ok(outcome) => Ok(outcome),
            Err(e) => Err(e),
        }
    }

    pub fn get_user_id(&self, signing_key: &str) -> Option<EntityID> {
        self.users_by_key.get(signing_key).cloned()
    }

    pub fn get_power_plant_cost(
        &mut self,
        scale: u64,
        location: (i64, i64),
        plant_type: PlantType,
        signing_key: String,
    ) -> Result<Vec<(String, u128, u128)>, TradeError> {
        let owner = self.users_by_key.get(&signing_key).unwrap().to_owned();
        PowerPlantProducer::estimate_materials_cost(self, scale, location, plant_type, owner)
    }
}

pub mod game_move;
