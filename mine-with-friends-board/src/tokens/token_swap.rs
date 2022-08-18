use super::TokenBase;
use super::TokenPointer;
use crate::entity::EntityID;
use crate::game::GameBoard;
use crate::tokens::TokenRegistry;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub(crate) struct ConstantFunctionMarketMakerPair {
    pub(crate) pair: TradingPairID,
    pub(crate) id: EntityID,
    pub(crate) lp: TokenPointer,
}

impl ConstantFunctionMarketMakerPair {
    fn ensure(game: &mut GameBoard, mut pair: TradingPairID) -> TradingPairID {
        pair.normalize();
        match game.swap.markets.entry(pair) {
            std::collections::btree_map::Entry::Vacant(_a) => {
                let name_a = game.tokens[pair.asset_a]
                    .nickname()
                    .unwrap_or(format!("{}", pair.asset_a.inner()));
                let name_b = game.tokens[pair.asset_b]
                    .nickname()
                    .unwrap_or(format!("{}", pair.asset_b.inner()));
                let base_id = game.alloc();
                let id = game.alloc();
                game.swap.markets.insert(
                    pair,
                    ConstantFunctionMarketMakerPair {
                        pair,
                        id,
                        lp: game.tokens.new_token(Box::new(TokenBase {
                            balances: Default::default(),
                            total: Default::default(),
                            this: base_id,
                            #[cfg(test)]
                            in_transaction: None,
                            nickname: Some(format!("swap({},{})::shares", name_a, name_b)),
                        })),
                    },
                );
                pair
            }
            std::collections::btree_map::Entry::Occupied(_a) => pair,
        }
    }
    fn amt_a(&self, tokens: &mut TokenRegistry) -> u128 {
        tokens[self.pair.asset_a].balance_check(&self.id)
    }
    fn amt_b(&self, tokens: &mut TokenRegistry) -> u128 {
        tokens[self.pair.asset_b].balance_check(&self.id)
    }
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TradingPairID {
    pub asset_a: TokenPointer,
    pub asset_b: TokenPointer,
}

impl TradingPairID {
    fn normalize(&mut self) {
        if self.asset_a <= self.asset_b {
        } else {
            *self = Self {
                asset_a: self.asset_b,
                asset_b: self.asset_a,
            }
        }
    }
}

#[derive(Serialize, Default)]
pub(crate) struct ConstantFunctionMarketMaker {
    pub(crate) markets: BTreeMap<TradingPairID, ConstantFunctionMarketMakerPair>,
}

impl ConstantFunctionMarketMaker {
    // TODO: Better math in this whole module

    pub(crate) fn deposit(
        game: &mut GameBoard,
        mut id: TradingPairID,
        mut amount_a: u128,
        mut amount_b: u128,
        from: EntityID,
    ) {
        let unnormalized_id = id;
        id.normalize();
        if id != unnormalized_id {
            std::mem::swap(&mut amount_a, &mut amount_b);
        }
        if amount_a == 0 || amount_b == 0 {
            return;
        }
        let id = ConstantFunctionMarketMakerPair::ensure(game, id);
        let mkt = &game.swap.markets[&id];

        let tokens: &mut TokenRegistry = &mut game.tokens;
        tokens[id.asset_a].transaction();
        tokens[id.asset_b].transaction();

        //        amount_a / amount_b = mkt.amt_a / mkt.amt_b
        if amount_a * mkt.amt_b(tokens) != mkt.amt_a(tokens) * amount_b {
            // todo: does the above need slack?
            return;
        }

        if !tokens[id.asset_a].transfer(&from, &mkt.id, amount_a) {
            return;
        }

        if !tokens[id.asset_b].transfer(&from, &mkt.id, amount_b) {
            // attempt to return asset a if asset b transfer fails...
            // if the return transfer fails then??
            let _ = tokens[id.asset_a].transfer(&mkt.id, &from, amount_a);
            return;
        }

        let coins = tokens[mkt.lp].total_coins();

        let to_mint = (coins * amount_a) / mkt.amt_a(tokens);

        let lp_tokens = &mut tokens[mkt.lp];
        lp_tokens.transaction();
        lp_tokens.mint(&from, to_mint);
        lp_tokens.end_transaction();
        tokens[id.asset_a].end_transaction();
        tokens[id.asset_b].end_transaction();
    }
    pub(crate) fn withdraw(&mut self) {
        todo!();
    }
    // One of amount_a or amount_b should be 0
    pub(crate) fn do_trade(
        game: &mut GameBoard,
        mut id: TradingPairID,
        mut amount_a: u128,
        mut amount_b: u128,
        from: EntityID,
    ) {
        let unnormalized_id = id;
        id.normalize();
        if id != unnormalized_id {
            std::mem::swap(&mut amount_a, &mut amount_b);
        }
        // the zero is the one to be computed
        if !(amount_a == 0 || amount_b == 0) {
            return;
        }
        let id = ConstantFunctionMarketMakerPair::ensure(game, id);
        let mkt = &game.swap.markets[&id];
        let tokens: &mut TokenRegistry = &mut game.tokens;
        tokens[id.asset_a].transaction();
        tokens[id.asset_b].transaction();
        if !(tokens[id.asset_a].balance_check(&from) >= amount_a
            && tokens[id.asset_b].balance_check(&from) >= amount_b)
        {
            return;
        }

        if !(amount_a <= mkt.amt_a(tokens) && amount_b <= mkt.amt_b(tokens)) {
            return;
        }

        if amount_a == 0 {
            let new_amount_a = (mkt.amt_a(tokens) * amount_b) / mkt.amt_b(tokens);
            let _ = tokens[id.asset_b].transfer(&from, &mkt.id, amount_b);
            let _ = tokens[id.asset_a].transfer(&mkt.id, &from, new_amount_a);
        } else {
            let new_amount_b = (mkt.amt_b(tokens) * amount_a) / mkt.amt_a(tokens);
            let _ = tokens[id.asset_a].transfer(&from, &mkt.id, amount_a);
            let _ = tokens[id.asset_b].transfer(&mkt.id, &from, new_amount_b);
        }

        tokens[id.asset_a].end_transaction();
        tokens[id.asset_b].end_transaction();
    }
}
