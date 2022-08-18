use std::cmp::min;

use serde::Serialize;

use crate::callbacks::Callback;
use crate::entity::EntityID;
use crate::tokens::TokenPointer;

use crate::tokens::token_swap::{TradingPairID, ConstantFunctionMarketMaker};
use crate::util::Price;

#[derive(Serialize)]
pub struct HashBoardData {
    pub hash_per_watt: u128,
    // out of 100, currently not used for anything.
    // Could be used to determine "burn out"
    pub reliability: u8,
}
#[derive(Serialize, Clone)]
pub struct ASICProducer {
    pub id: EntityID,
    pub total_units: u128,
    pub base_price: Price,
    pub price_asset: TokenPointer,
    pub hash_asset: TokenPointer,
    pub adjusts_every: u64,
    pub current_time: u64,
    pub first: bool,
}

impl Callback for ASICProducer {
    fn time(&self) -> u64 {
        self.current_time
    }

    fn action(&mut self, game: &mut crate::game::GameBoard) {
        let pair = TradingPairID {
            asset_a: self.hash_asset,
            asset_b: self.price_asset,
        };
        if self.first {
            {
                let coin = &mut game.tokens[self.hash_asset];
                coin.transaction();
                coin.mint(&self.id, self.total_units);
                coin.end_transaction();
            }
            let start = self.total_units / 10;
            let base = self.base_price * start;
            {
                let coin = &mut game.tokens[self.price_asset];
                coin.transaction();
                coin.mint(&self.id, base);
                coin.end_transaction();
            }
            ConstantFunctionMarketMaker::deposit(game, pair, start, base, self.id);
            self.first = false;
            self.total_units -= start;
        }
        let balance = game.tokens[self.hash_asset].balance_check(&self.id);
        // TODO: Something more clever here?
        ConstantFunctionMarketMaker::do_trade(game, pair, min(balance / 100, balance), 0, self.id);

        self.current_time += self.adjusts_every;
        let balance = game.tokens[self.hash_asset].balance_check(&self.id);
        if balance > 0 {
            game.callbacks.schedule(Box::new(self.clone()))
        }
    }

    fn purpose(&self) -> String {
        format!("Adjusting the market for ASICs")
    }
}
