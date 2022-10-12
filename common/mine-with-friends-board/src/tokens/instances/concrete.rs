// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    callbacks::Callback,
    entity::EntityID,
    game::CallContext,
    tokens::{
        token_swap::{ConstantFunctionMarketMaker, TradingPairID},
        TokenPointer,
    },
    util::Price,
};
use serde::Serialize;
use std::cmp::min;

#[derive(Serialize, Clone, Debug)]
pub struct ConcreteMiller {
    pub id: EntityID,
    pub total_units: u128,
    pub base_price: Price,
    pub price_asset: TokenPointer,
    pub hash_asset: TokenPointer,
    pub adjusts_every: u64,
    pub elapsed_time: u64,
    pub first: bool,
}

impl Callback for ConcreteMiller {
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
                let unit = &mut game.tokens[self.hash_asset];
                unit.transaction();
                unit.mint(&self.id, self.total_units);
                unit.end_transaction();
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
        ConstantFunctionMarketMaker::do_sell_trade(
            game,
            pair,
            min(balance / 100, balance),
            0,
            None,
            false,
            &CallContext { sender: self.id },
        );

        self.elapsed_time = game.elapsed_time + self.adjusts_every;
        let balance = game.tokens[self.hash_asset].balance_check(&self.id);
        if balance > 0 {
            game.callbacks.schedule(Box::new(self.clone()))
        }
    }

    fn purpose(&self) -> String {
        "Releasing new Concrete to the market".to_string()
    }
}
