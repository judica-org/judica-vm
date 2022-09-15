use std::cmp::min;

use serde::Serialize;

pub mod events;
use crate::{game::GameBoard, nfts::NftPtr, tokens::TokenPointer, util::Price};

use super::lockup::CoinLockup;

#[derive(Serialize, Clone)]
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
