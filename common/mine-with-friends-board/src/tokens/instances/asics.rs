///! Tokens which represent 1 unit of hashrate
use std::cmp::min;

use serde::Serialize;

use crate::callbacks::Callback;
use crate::entity::EntityID;
use crate::game::CallContext;
use crate::tokens::TokenPointer;

use crate::tokens::token_swap::{ConstantFunctionMarketMaker, TradingPairID};
use crate::util::Price;

/// Parameters for a given HashBoard type
#[derive(Serialize)]
pub struct HashBoardData {
    pub hash_per_watt: u128,
    // out of 100, currently not used for anything.
    // Could be used to determine "burn out"
    pub reliability: u8,
}

/// A ASICProducer is a kind of CFMM bot that deploys a basic strategy to
/// periodically sell 1% of it's holdings in ASICs, after setting up an initial
/// market condition with 10% of the hashrate available at a set price.
///
/// If it were more clever, the algorithm could do some fancier things.
#[derive(Serialize, Clone)]
pub struct ASICProducer {
    pub id: EntityID,
    pub total_units: u128,
    pub base_price: Price,
    pub price_asset: TokenPointer,
    pub hash_asset: TokenPointer,
    pub adjusts_every: u64,
    pub elapsed_time: u64,
    // On our first round we'll set some things up differently
    pub first: bool,
}

impl Callback for ASICProducer {
    fn time(&self) -> u64 {
        self.elapsed_time
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
        //
        // Ideas:
        // if the current price is below the base price, then do not sell.  if
        // the current price is above the current price, then sell enough to get
        // the price back to base_price.
        // Maybe worth implementing logic inside the swap contract directly for these.
        ConstantFunctionMarketMaker::do_sell_trade(
            game,
            pair,
            min(balance / 100, balance),
            0,
            false,
            &CallContext { sender: self.id },
        );

        self.elapsed_time += self.adjusts_every;
        let balance = game.tokens[self.hash_asset].balance_check(&self.id);
        if balance > 0 {
            game.callbacks.schedule(Box::new(self.clone()))
        }
    }

    fn purpose(&self) -> String {
        "Adjusting the market for ASICs".to_string()
    }
}
