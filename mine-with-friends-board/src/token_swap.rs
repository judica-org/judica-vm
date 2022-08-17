use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::entity::EntityIDAllocator;
use crate::erc20::ERC20Registry;

use super::entity::EntityID;
use super::erc20;

#[derive(Serialize)]
pub(crate) struct UniswapPair {
    pub(crate) pair: PairID,
    pub(crate) id: EntityID,
    pub(crate) lp: erc20::ERC20Ptr,
}

impl UniswapPair {
    fn new(alloc: &mut EntityIDAllocator, tokens: &mut ERC20Registry, pair: PairID) -> UniswapPair {
        UniswapPair {
            pair,
            id: alloc.make(),
            lp: tokens.new_token(Box::new(erc20::ERC20Standard {
                balances: Default::default(),
                total: Default::default(),
                this: alloc.make(),
                #[cfg(test)]
                in_transaction: None,
            })),
        }
    }
    fn amt_a(&self, tokens: &mut ERC20Registry) -> u128 {
        tokens[self.pair.asset_a].balance_check(&self.id)
    }
    fn amt_b(&self, tokens: &mut ERC20Registry) -> u128 {
        tokens[self.pair.asset_b].balance_check(&self.id)
    }
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PairID {
    pub asset_a: erc20::ERC20Ptr,
    pub asset_b: erc20::ERC20Ptr,
}

impl PairID {
    pub fn normalize(&mut self) {
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
pub(crate) struct Uniswap {
    pub(crate) markets: BTreeMap<PairID, UniswapPair>,
}

impl Uniswap {
    // TODO: Better math in this whole module

    pub(crate) fn deposit(
        &mut self,
        tokens: &mut erc20::ERC20Registry,
        alloc: &mut EntityIDAllocator,
        id: PairID,
        amount_a: u128,
        amount_b: u128,
        from: EntityID,
    ) {
        tokens[id.asset_a].transaction();
        tokens[id.asset_b].transaction();
        if amount_a == 0 || amount_b == 0 {
            return;
        }
        let mut mkt = self
            .markets
            .entry(id)
            .or_insert_with(|| UniswapPair::new(alloc, tokens, id));

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
        &mut self,
        tokens: &mut erc20::ERC20Registry,
        alloc: &mut EntityIDAllocator,
        id: PairID,
        amount_a: u128,
        amount_b: u128,
        from: EntityID,
    ) {
        tokens[id.asset_a].transaction();
        tokens[id.asset_b].transaction();
        // the zero is the one to be computed
        if !(amount_a == 0 || amount_b == 0) {
            return;
        }

        if !(tokens[id.asset_a].balance_check(&from) >= amount_a
            && tokens[id.asset_b].balance_check(&from) >= amount_b)
        {
            return;
        }

        let mut mkt = self
            .markets
            .entry(id)
            .or_insert_with(|| UniswapPair::new(alloc, tokens, id));

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
