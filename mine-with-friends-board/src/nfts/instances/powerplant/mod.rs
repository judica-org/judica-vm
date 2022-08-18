use std::cmp::min;

use serde::Serialize;

pub mod events;
use crate::{
    entity::EntityIDAllocator,
    game::GameBoard,
    nfts::{BaseNFT, NFTRegistry, NftPtr},
    tokens::{TokenPointer, TokenRegistry},
    util::Price,
};

use super::lockup::CoinLockup;

#[derive(Serialize, Clone)]
pub enum PlantType {
    Coal,
    Solar,
    Hydro,
    Nuclear,
    Geothermal,
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
    fn compute_hashrate(&self, game: &mut GameBoard) -> u128 {
        // TODO: Some algo that uses watts / coordinates / plant_type to compute a scalar?
        let _scale = 1000;
        let mut hash = Vec::with_capacity(game.tokens.hashboards.len());
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
    fn colocate_hashrate(&self, game: &mut GameBoard, miners: TokenPointer, amount: Price) {
        let owner = game.nfts[self.id].owner();
        game.tokens[miners].transaction();
        let _ = game.tokens[miners].transfer(&owner, &self.id.0, amount);
        game.tokens[miners].end_transaction();
    }
    /// Withdrawals are processed via a CoinLockup which emulates shipping
    fn ship_hashrate(
        &self,
        tokens: &mut TokenRegistry,
        miners: TokenPointer,
        amount: Price,
        nfts: &mut NFTRegistry,
        alloc: &mut EntityIDAllocator,
        shipping_time: u64,
        game: &mut GameBoard,
    ) {
        let owner = game.nfts[self.id].owner();
        let lockup_base = BaseNFT {
            owner: owner,
            nft_id: alloc.make(),
            // Non Transferrable
            transfer_count: u128::max_value(),
        };
        let id = nfts.add(Box::new(lockup_base.clone()));
        let lockup = CoinLockup {
            time_when_free: shipping_time,
            asset: miners,
            id,
        };
        game.callbacks.schedule(Box::new(lockup.clone()));
        tokens[miners].transaction();
        let _ = tokens[miners].transfer(&self.id.0, &id.0, amount);
        tokens[miners].end_transaction();
    }
}
