use std::cmp::min;

pub mod events;
use crate::{game::GameBoard, nfts::NftPtr, tokens::TokenPointer, util::Price};
use super::BaseNFT;
use crate::entity::EntityID;
use crate::game::GameBoard;
use crate::util::Currency;
use crate::util::Price;
use super::lockup::CoinLockup;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
pub enum PlantType {
    Solar,
    Hydro,
    Flare,
}
#[derive(Serialize, Clone)]
pub(crate) struct PowerPlant {
    pub id: NftPtr,
    pub plant_type: PlantType,
    pub watts: u128,
    pub coordinates: (u64, u64),
}

impl PowerPlant {
    /// Create a new PowerPlant
    fn new(id: NftPtr, plant_type: PlantType, coordinates: (u64, u64)) -> Self {
        Self {
            id,
            plant_type,
            watts: 1000, // this should probably change
            coordinates,
        }
    }
    /// Mint a new PowerPlant NFT
    pub(crate) fn mint_power_plant(
        &self,
        game: &mut GameBoard,
        // need to put a power plant price map somewhere
        resources: Vec<(Currency, Price)>,
        location: (u64, u64),
        plant_type: PlantType,
        owner: EntityID,
    ) {
        // check whether owner has enough of each material
        // there's a better way to do this
        let mut insufficient = false;
        for (currency, price) in resources {
            let token = &mut game.tokens[currency];
            token.transaction();
            if token.balance_check(&owner) < price {
                insufficient = true;
            }
            token.end_transaction();
        }
        if insufficient {
            return;
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
        let new_plant = self.new(plant_ptr, plant_type, location);
        // add to plant register, need to return Plant?
        let _ = game.nfts.power_plants.insert(plant_ptr, new_plant).unwrap();

        // exchange (or burn?) tokens
        for (currency, price) in resources {
            let token = &mut game.tokens[currency];
            token.transaction();
            let _ = token.transfer(&owner, &plant_ptr.0, price);
            token.end_transaction();
        }
    }
    /// Compute the total hashes per second of this powerplant at this game state
    pub(crate) fn compute_hashrate(&self, game: &mut GameBoard) -> u128 {
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
    fn ship_hashrate(
        &self,
        miners: TokenPointer,
        amount: Price,
        shipping_time: u64,
        game: &mut GameBoard,
    ) {
        let owner = game.nfts[self.id].owner();
        CoinLockup::lockup(game, owner, miners, amount, shipping_time)
    }
}
