use self::game_move::Chat;
use self::game_move::GameMove;
use self::game_move::Heartbeat;
use self::game_move::ListNFTForSale;
use self::game_move::MintPowerPlant;
use self::game_move::PurchaseNFT;
use self::game_move::SendTokens;
use self::game_move::Trade;
use crate::callbacks::CallbackRegistry;
use crate::entity::EntityID;
use crate::entity::EntityIDAllocator;
use crate::nfts::instances::powerplant::events::PowerPlantEvent;
use crate::nfts::instances::powerplant::PlantType;
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
use std::ops::Index;
use tokens::TokenBase;
use tokens::TokenPointer;
use tokens::TokenRegistry;
use tracing::info;

#[derive(Serialize, Clone, Debug)]
pub struct UXUserInventory {
    user_power_plants: BTreeMap<NftPtr, UXPlantData>,
    user_token_balances: Vec<(String, u128)>,
}
#[derive(Serialize)]
pub struct UserData {
    key: String,
}
/// GameBoard holds the entire state of the game.
#[derive(Serialize)]
pub struct GameBoard {
    pub(crate) tokens: tokens::TokenRegistry,
    pub(crate) swap: token_swap::ConstantFunctionMarketMaker,
    /// Make this a vote over the map of users to current vote and let the turn count be dynamic
    pub(crate) turn_count: u64,
    alloc: EntityIDAllocator,
    pub(crate) users: BTreeMap<EntityID, UserData>,
    pub(crate) users_by_key: BTreeMap<String, EntityID>,
    pub(crate) nfts: NFTRegistry,
    pub(crate) nft_sales: NFTSaleRegistry,
    pub(crate) player_move_sequence: BTreeMap<EntityID, u64>,
    /// If init = true, must be Some
    pub(crate) bitcoin_token_id: TokenPointer,
    /// If init = true, must be Some
    pub(crate) dollar_token_id: TokenPointer,
    /// If init = true, must be Some
    pub(crate) steel_token_id: TokenPointer,
    /// If init = true, must be Some
    pub(crate) silicon_token_id: TokenPointer,
    /// If init = true, must be Some
    pub(crate) concrete_token_id: TokenPointer,
    /// If init = true, must be Some
    pub(crate) root_user: EntityID,
    pub(crate) callbacks: CallbackRegistry,
    pub(crate) current_time: u64,
    pub(crate) mining_subsidy: u128,
    pub ticks: BTreeMap<EntityID, u64>,
    pub chat: VecDeque<(u64, EntityID, String)>,
    pub chat_counter: u64,
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
}
impl GameSetup {
    fn setup_game(&self, g: &mut GameBoard) {
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
            g.tokens[g.dollar_token_id].mint(&id, self.start_amount as u128);
        }
    }
}

impl GameBoard {
    /// Creates a new GameBoard
    pub fn new(setup: &GameSetup) -> GameBoard {
        let mut alloc = EntityIDAllocator::new();

        let btc = Box::new(TokenBase::new_from_alloc(&mut alloc, "Bitcoin".into()));
        let dollar = Box::new(TokenBase::new_from_alloc(&mut alloc, "US Dollar".into()));
        let concrete = Box::new(TokenBase::new_from_alloc(&mut alloc, "Concrete".into()));
        let asic = Box::new(TokenBase::new_from_alloc(&mut alloc, "ASIC Gen 1".into()));
        let steel = Box::new(TokenBase::new_from_alloc(&mut alloc, "Steel".into()));
        let silicon = Box::new(TokenBase::new_from_alloc(&mut alloc, "Silicon".into()));
        let mut tokens = TokenRegistry::default();
        let bitcoin_token_id = tokens.new_token(btc);
        let dollar_token_id = tokens.new_token(dollar);
        let concrete_token_id = tokens.new_token(concrete);
        let steel_token_id = tokens.new_token(steel);
        let silicon_token_id = tokens.new_token(silicon);
        let asic_token_id = tokens.new_token(asic);
        tokens.hashboards.insert(
            asic_token_id,
            HashBoardData {
                hash_per_watt: (3.0 * 10e12) as u128,
                reliability: 100,
            },
        );
        tokens.steel.insert(
            steel_token_id,
            Steel {
                variety: tokens::instances::steel::SteelVariety::Structural,
                weight_in_kg: 1,
            },
        );
        tokens
            .silicon
            .insert(silicon_token_id, Silicon { weight_in_kg: 1 });

        let root_user = alloc.make();
        let mut plant_prices = HashMap::new();
        plant_prices.insert(
            PlantType::Solar,
            Vec::from([
                (steel_token_id, 57),
                (silicon_token_id, 437),
                (concrete_token_id, 62),
            ]),
        );
        plant_prices.insert(
            PlantType::Hydro,
            Vec::from([
                (steel_token_id, 247),
                (silicon_token_id, 96),
                (concrete_token_id, 144),
            ]),
        );
        plant_prices.insert(
            PlantType::Flare,
            Vec::from([
                (steel_token_id, 76),
                (silicon_token_id, 84),
                (concrete_token_id, 54),
            ]),
        );

        let mut g = GameBoard {
            tokens,
            swap: Default::default(),
            turn_count: 0,
            bitcoin_token_id,
            dollar_token_id,
            steel_token_id,
            silicon_token_id,
            concrete_token_id,
            root_user,
            alloc,
            users: Default::default(),
            users_by_key: Default::default(),
            nfts: Default::default(),
            nft_sales: Default::default(),
            player_move_sequence: Default::default(),
            callbacks: Default::default(),
            current_time: 0,
            mining_subsidy: 100_000_000_000 * 50,
            ticks: Default::default(),
            chat: VecDeque::with_capacity(1000),
            chat_counter: 0,
            plant_prices,
        };
        g.post_init();
        setup.setup_game(&mut g);
        g
    }

    fn post_init(&mut self) {
        // DEMO CODE:
        // REMOVE BEFORE FLIGHT
        self.tokens[self.bitcoin_token_id].mint(&self.root_user, 10000000);
        self.tokens[self.dollar_token_id].mint(&self.root_user, 30000);
        //
        let id = self.alloc();
        self.callbacks.schedule(Box::new(ASICProducer {
            id,
            total_units: 100_000,
            base_price: 20,
            price_asset: self.bitcoin_token_id,
            hash_asset: *self.tokens.hashboards.iter().next().unwrap().0,
            adjusts_every: 100, // what units?
            current_time: 0,
            first: true,
        }));
        let steel_id = self.alloc();
        self.callbacks.schedule(Box::new(SteelSmelter {
            id: steel_id,
            total_units: 100_000,
            base_price: 1,
            price_asset: self.bitcoin_token_id,
            hash_asset: self.steel_token_id,
            adjusts_every: 100, // what units?
            current_time: 0,
            first: true,
        }));
        let silicon_id = self.alloc();
        self.callbacks.schedule(Box::new(SiliconRefinery {
            id: silicon_id,
            total_units: 100_000,
            base_price: 38,
            price_asset: self.bitcoin_token_id,
            hash_asset: self.silicon_token_id,
            adjusts_every: 100, // what units?
            current_time: 0,
            first: true,
        }));
        let concrete_id = self.alloc();
        self.callbacks.schedule(Box::new(ConcreteMiller {
            id: concrete_id,
            total_units: 100_000,
            base_price: 290,
            price_asset: self.bitcoin_token_id,
            hash_asset: self.concrete_token_id,
            adjusts_every: 100, // what units?
            current_time: 0,
            first: true,
        }));
        // TODO: Initialize Power Plants?
        let nft_id = self.alloc();
        let demo_nft = self.nfts.add(Box::new(BaseNFT {
            owner: self.root_user,
            nft_id,
            transfer_count: 0,
        }));
        self.nft_sales.list_nft(
            &CallContext {
                sender: self.root_user,
            },
            demo_nft,
            1000,
            self.bitcoin_token_id,
            &self.nfts,
        );
        self.callbacks.schedule(Box::new(PowerPlantEvent {
            time: self.current_time + 100,
            period: 100,
        }));
    }
    /// Creates a new EntityID
    pub fn alloc(&mut self) -> EntityID {
        self.alloc.make()
    }
    pub fn root_user(&self) -> EntityID {
        self.root_user
    }
    /// Check if a given user is the root_user
    pub fn user_is_admin(&self, user: EntityID) -> bool {
        user == self.root_user
    }

    /// Processes a GameMove against the board after verifying it's integrity
    /// and sanitizing it.
    pub fn play(
        &mut self,
        MoveEnvelope { d, sequence, time }: MoveEnvelope,
        signed_by: String,
    ) -> Result<(), ()> {
        let from = *self.users_by_key.get(&signed_by).ok_or(())?;
        info!(key = signed_by, ?from, "Got Move {} From Player", sequence);
        // TODO: check that sequence is the next sequence for that particular user
        let current_move = self.player_move_sequence.entry(from).or_default();
        if (*current_move + 1) != sequence || *current_move == 0 && sequence == 0 {
            return Ok(());
        } else {
            *current_move = sequence;
        }
        let mv = d.sanitize(())?;
        self.update_current_time(Some((from, time)));
        self.process_ticks();

        // TODO: verify the key/sig/d combo (or it happens during deserialization of Verified)
        self.play_inner(mv, from)?;
        info!("Move Successfully Made");
        Ok(())
    }

    pub fn process_ticks(&mut self) {
        CallbackRegistry::run(self);
    }

    fn update_current_time(&mut self, update_from: Option<(EntityID, u64)>) {
        if let Some((from, time)) = update_from {
            let tick = self.ticks.entry(from).or_default();
            *tick = max(*tick, time);
        }
        let mut ticks: Vec<u64> = self.ticks.values().cloned().collect();
        ticks.sort_unstable();
        let median_time = ticks.get(ticks.len() / 2).cloned().unwrap_or_default();
        self.current_time = median_time;
    }

    /// Processes a GameMove without any sanitization
    pub fn play_inner(&mut self, d: GameMove, from: EntityID) -> Result<(), ()> {
        // TODO: verify the key/sig/d combo (or it happens during deserialization of Verified)
        let context = CallContext { sender: from };
        match d {
            GameMove::Heartbeat(Heartbeat()) => {}
            GameMove::Trade(Trade {
                pair,
                amount_a,
                amount_b,
            }) => {
                ConstantFunctionMarketMaker::do_sell_trade(
                    self, pair, amount_a, amount_b, false, &context,
                );
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
                );
            }
            GameMove::SuperMintPowerPlant(MintPowerPlant {
                scale,
                location,
                plant_type,
            }) => {
                PowerPlantProducer::super_mint(self, scale, location, plant_type, context.sender);
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
            GameMove::Chat(Chat(s)) => {
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

    pub fn get_ux_materials_prices(&mut self) -> Result<Vec<UXMaterialsPriceData>, ()> {
        let mut price_data = Vec::new();
        // get pointer and human name for materials and
        let bitcoin_token_id = self.bitcoin_token_id;
        let steel_token_id = self.steel_token_id;
        let silicon_token_id = self.silicon_token_id;
        let concrete_token_id = self.concrete_token_id;
        // get ux names
        let registry = &self.tokens;
        let human_name_bitcoin = registry
            .index(bitcoin_token_id)
            .nickname()
            .unwrap_or_else(|| "Bitcoin".into());
        let human_name_steel = registry
            .index(steel_token_id)
            .nickname()
            .unwrap_or_else(|| "Steel".into());
        let human_name_silicon = registry
            .index(silicon_token_id)
            .nickname()
            .unwrap_or_else(|| "Silicon".into());
        let human_name_concrete = registry
            .index(concrete_token_id)
            .nickname()
            .unwrap_or_else(|| "Concrete".into());
        // get steel/btc
        let (steel_qty_btc, btc_qty_steel) = ConstantFunctionMarketMaker::get_pair_price_data(
            self,
            TradingPairID {
                asset_a: steel_token_id,
                asset_b: bitcoin_token_id,
            },
        )
        .unwrap();
        // get silicon/btc
        let (silicon_qty_btc, btc_qty_silicon) = ConstantFunctionMarketMaker::get_pair_price_data(
            self,
            TradingPairID {
                asset_a: silicon_token_id,
                asset_b: bitcoin_token_id,
            },
        )
        .unwrap();
        // get concrete/btc
        let (concrete_qty_btc, btc_qty_concrete) =
            ConstantFunctionMarketMaker::get_pair_price_data(
                self,
                TradingPairID {
                    asset_a: concrete_token_id,
                    asset_b: bitcoin_token_id,
                },
            )
            .unwrap();

        price_data.push(UXMaterialsPriceData {
            trading_pair: TradingPairID {
                asset_a: steel_token_id,
                asset_b: bitcoin_token_id,
            },
            asset_a: human_name_steel,
            mkt_qty_a: steel_qty_btc,
            asset_b: human_name_bitcoin.clone(),
            mkt_qty_b: btc_qty_steel,
        });
        price_data.push(UXMaterialsPriceData {
            trading_pair: TradingPairID {
                asset_a: silicon_token_id,
                asset_b: bitcoin_token_id,
            },
            asset_a: human_name_silicon,
            mkt_qty_a: silicon_qty_btc,
            asset_b: human_name_bitcoin.clone(),
            mkt_qty_b: btc_qty_silicon,
        });
        price_data.push(UXMaterialsPriceData {
            trading_pair: TradingPairID {
                asset_a: concrete_token_id,
                asset_b: bitcoin_token_id,
            },
            asset_a: human_name_concrete,
            mkt_qty_a: concrete_qty_btc,
            asset_b: human_name_bitcoin,
            mkt_qty_b: btc_qty_concrete,
        });

        Ok(price_data)
    }

    // where does miner status come from
    pub fn get_ux_power_plant_data(&self) -> Vec<(crate::nfts::NftPtr, UXPlantData)> {
        let mut power_plant_data = Vec::new();
        let plants = &self.nfts.power_plants.clone();
        plants.iter().for_each(|(pointer, power_plant)| {
            let mut for_sale = false;
            if let Some(_nft_sale) = &self.nft_sales.nfts.get(pointer) {
                for_sale = true;
            }
            // unwrap should be safe here - we have problems if we have a pointer and cant find the NFT.
            let owner = &self.nfts.nfts.get(pointer).unwrap().owner();

            power_plant_data.push((
                *pointer,
                UXPlantData {
                    coordinates: power_plant.coordinates,
                    for_sale,
                    has_miners: false,
                    owner: *owner,
                    plant_type: power_plant.plant_type.clone(),
                    watts: power_plant.watts,
                    hashrate: power_plant.compute_hashrate(self),
                },
            ));
        });
        power_plant_data
    }
    // how do we tell whether hashbox is colocated?
    pub fn get_all_power_plants(&mut self) -> Result<UXNFTRegistry, ()> {
        let mut power_plant_data = BTreeMap::new();

        let power_plant_vec = self.get_ux_power_plant_data();
        power_plant_vec.iter().for_each(|(ptr, plant)| {
            power_plant_data.insert(*ptr, plant.clone());
        });

        Ok(UXNFTRegistry { power_plant_data })
    }

    pub fn get_user_power_plants(&self, user_id: EntityID) -> Result<UXNFTRegistry, ()> {
        let mut power_plant_data = BTreeMap::new();
        let mut power_plant_vec = self.get_ux_power_plant_data();
        // should use something other than drain_filter?
        power_plant_vec.retain(|(_ptr, plant)| plant.owner.eq(&user_id));
        power_plant_vec.iter().for_each(|(ptr, plant)| {
            power_plant_data.insert(*ptr, plant.clone());
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
    pub fn get_user_hashrate_share(&self) -> BTreeMap<EntityID, (u64, u64)> {
        let denominator = 100000u64;
        let reg = &self.nfts;
        let mut res = BTreeMap::new();
        let mut total = 0u64;
        // accumulation step
        for (ptr, plant) in reg.power_plants.iter() {
            let rate = plant.compute_hashrate(self) as u64;
            let player = reg.nfts.get(ptr).unwrap().owner();
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
        res.iter_mut().for_each(|(_, v)| v.0 /= total);
        res
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
            true,
            &CallContext { sender },
        ) {
            Ok(outcome) => Ok(outcome),
            Err(e) => Err(e),
        }
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
