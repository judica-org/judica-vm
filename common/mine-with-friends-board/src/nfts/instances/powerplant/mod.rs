use std::cmp::min;
use std::collections::HashMap;
pub mod events;
use super::lockup::CoinLockup;
use crate::entity::EntityID;
use crate::game::CallContext;
use crate::game::GameBoard;
use crate::tokens::token_swap::ConstantFunctionMarketMaker;
use crate::tokens::token_swap::TradeError;
use crate::tokens::token_swap::TradingPairID;

use crate::util::Currency;
use crate::util::Price;
use crate::{nfts::BaseNFT, nfts::NftPtr, tokens::TokenPointer};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) type PowerPlantPrices = HashMap<PlantType, Vec<(Currency, Price)>>;

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, JsonSchema, Hash, Copy)]
pub enum PlantType {
    Solar,
    Hydro,
    Flare,
}

impl PlantType {
    fn raw_materials_bill(&self, game: &GameBoard, scale: u64) -> Vec<(Currency, Price)> {
        let base_prices = game.plant_prices.get(self).unwrap().to_owned();
        let total_prices = base_prices
            .iter()
            .map(|(cur, qty)| (*cur, *qty * scale as u128))
            .clone()
            .collect();
        total_prices
    }
}
#[derive(Serialize, Clone)]
pub(crate) struct PowerPlant {
    pub id: NftPtr,
    pub plant_type: PlantType,
    pub watts: u128,
    pub coordinates: (i64, i64),
}

impl PowerPlant {
    /// Create a new PowerPlant
    fn new(
        game: &GameBoard,
        id: NftPtr,
        plant_type: PlantType,
        coordinates: (i64, i64),
        scale: u64,
    ) -> Self {
        let watts = {
            // this can be a more fun calculation in the future
            let materials = plant_type.raw_materials_bill(game, scale);
            let mut total_watts = 0;
            for (_, qty) in materials {
                total_watts += qty / 2;
            }
            total_watts
        };
        Self {
            id,
            plant_type,
            watts,
            coordinates,
        }
    }

    /// Compute the total hashes per second of this powerplant at this game state
    pub(crate) fn compute_hashrate(&self, game: &GameBoard) -> u128 {
        // TODO: Some algo that uses watts / coordinates / plant_type to compute a scalar?
        let _scale = 1000;
        let len = game.tokens.hashboards.len();
        let mut hash = Vec::with_capacity(len);
        let hashers: Vec<_> = game.tokens.hashboards.keys().cloned().collect();
        for token in hashers {
            if let Some(hbd) = game.tokens.hashboards.get(&token) {
                let hpw = hbd.hash_per_watt;
                let count = game.tokens[token].balance_check(&self.id.0);
                hash.push((hpw, count));
            }
        }
        hash.sort_unstable();
        let mut watts = self.watts;
        let mut hashrate = 0;
        while let Some((hpw, units)) = hash.pop() {
            let available = min(units, watts);
            hashrate += available * hpw;
            watts -= available;
            if watts == 0 {
                break;
            }
        }
        hashrate
    }

    /// Send some of a the owner's hash boxes to this powerplant
    fn colocate_hashrate(&self, game: &mut GameBoard, miners: TokenPointer, amount: Price) {
        let owner = game.nfts[self.id].owner();
        game.tokens[miners].transaction();
        let _ = game.tokens[miners].transfer(&owner, &self.id.0, amount);
        game.tokens[miners].end_transaction();
    }
    /// Withdrawals are processed via a CoinLockup which emulates shipping
    pub(crate) fn ship_hashrate(
        &self,
        miners: TokenPointer,
        amount: Price,
        shipping_time: u64,
        game: &mut GameBoard,
    ) {
        let owner = game.nfts[self.id].owner();
        let sender = self.id.0;
        CoinLockup::lockup(game, sender, owner, miners, amount, shipping_time)
    }
}

pub(crate) struct PowerPlantProducer {}

impl PowerPlantProducer {
    /// Mint a new PowerPlant NFT
    pub(crate) fn mint_power_plant(
        game: &mut GameBoard,
        // size of the plant
        scale: u64,
        location: (i64, i64),
        plant_type: PlantType,
        owner: EntityID,
    ) -> Result<(), TradeError> {
        let resources = plant_type.raw_materials_bill(game, scale);
        // check whether owner has enough of each material
        // there's a better way to do this
        for (currency, price) in &resources {
            let token = &mut game.tokens[*currency];
            if token.balance_check(&owner) < *price {
                return Err(TradeError::InsufficientTokens(format!(
                    "Balance Check Failed: {:?}",
                    currency
                )));
            }
        }
        // create base nft?
        let base_power_plant = BaseNFT {
            owner,
            nft_id: game.alloc(),
            transfer_count: 0,
        };
        // insert into registry and get pointer
        let plant_ptr = game.nfts.add(Box::new(base_power_plant));
        // create PowerPlant nft
        let new_plant = PowerPlant::new(game, plant_ptr, plant_type, location, scale);
        // add to plant register, need to return Plant?
        let _ = game.nfts.power_plants.insert(plant_ptr, new_plant);

        // exchange (or burn?) tokens
        for (currency, price) in resources {
            let token = &mut game.tokens[currency];
            token.transaction();
            let _ = token.transfer(&owner, &plant_ptr.0, price);
            token.end_transaction();
        }
        Ok(())
    }

    pub(crate) fn super_mint(
        game: &mut GameBoard,
        // size of the plant
        scale: u64,
        location: (i64, i64),
        plant_type: PlantType,
        owner: EntityID,
    ) -> Result<(), TradeError> {
        let cost_estimates =
            PowerPlantProducer::estimate_materials_cost(game, scale, location, plant_type, owner)?;
        tracing::trace!(?cost_estimates, "Plant Cost Estimate");
        let total_cost = cost_estimates.iter().map(|i| i.2).sum::<u128>();
        // get btc token ptr
        let btc_token_ptr = game.bitcoin_token_id;
        if game.tokens[btc_token_ptr].balance_check(&owner) < total_cost {
            return Err(TradeError::InsufficientTokens(format!(
                "Need at least {} of {:?} to mint",
                total_cost, btc_token_ptr
            )));
        }
        // get bill for plant
        let resources = plant_type.raw_materials_bill(game, scale);
        // need CallContext
        let ctx = CallContext { sender: owner };
        // for each resource/qty
        for (material, qty) in resources {
            // create a trading pair
            let id = TradingPairID {
                asset_a: material,      // purchasing
                asset_b: btc_token_ptr, // paying
            };
            // purchse material from mkt
            ConstantFunctionMarketMaker::do_buy_trade(game, id, qty, 0, None, false, &ctx)
                .expect("buy trade may not fail here given earlier estimate");
        }
        // mint the power plant
        PowerPlantProducer::mint_power_plant(game, scale, location, plant_type, owner)
    }

    /// returns the quantity and cost in BTC for each material needed to build a plant
    pub fn estimate_materials_cost(
        game: &mut GameBoard,
        scale: u64,
        _location: (i64, i64),
        plant_type: PlantType,
        owner: EntityID,
    ) -> Result<Vec<(String, u128, u128)>, TradeError> {
        let ctx = CallContext { sender: owner };
        // get bill for plant
        let resources = plant_type.raw_materials_bill(game, scale);
        let btc_token_ptr = game.bitcoin_token_id;
        // for each resource/qty
        let prices_in_btc: Vec<(String, u128, u128)> = resources
            .iter()
            .map(|(material, qty)| {
                let human_name: String = game.tokens[*material]
                    .nickname()
                    .unwrap_or_else(|| "Material Not Found".into());
                // create a trading pair
                let id = TradingPairID {
                    asset_a: *material,     // purchasing
                    asset_b: btc_token_ptr, // paying in
                };
                // simulate trade
                let cost_in_btc =
                    ConstantFunctionMarketMaker::do_buy_trade(game, id, *qty, 0, None, true, &ctx)?
                        .amount_player_sold;
                Ok((human_name, *qty, cost_in_btc))
            })
            .collect::<Result<_, _>>()?;
        Ok(prices_in_btc)
    }
}
