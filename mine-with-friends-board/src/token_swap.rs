use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use crate::erc20::ERC20Registry;
use crate::entity::EntityIDAllocator;

use super::erc20;
use super::entity::EntityID;

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
            lp: tokens.new_token(Box::new(erc20::ERC20Standard::default())),
        }
    }
    fn amt_a(&self, tokens: &mut ERC20Registry) -> u128 {
        tokens[self.pair.0].balance_check(&self.id)
    }
    fn amt_b(&self, tokens: &mut ERC20Registry) -> u128 {
        tokens[self.pair.1].balance_check(&self.id)
    }
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Serialize, Deserialize)]
pub struct PairID(pub erc20::ERC20Ptr, pub erc20::ERC20Ptr);

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
        tokens[id.0].transaction();
        tokens[id.1].transaction();
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

        if !(tokens[id.0].balance_check(&from) >= amount_a
            && tokens[id.1].balance_check(&from) >= amount_b)
        {
            return;
        }

        tokens[id.0].balance_sub(&from, amount_a);
        tokens[id.1].balance_sub(&from, amount_b);

        tokens[id.0].add_balance(&mkt.id, amount_a);
        tokens[id.1].add_balance(&mkt.id, amount_a);

        let coins = tokens[mkt.lp].total_coins();

        let to_mint = (coins * amount_a) / mkt.amt_a(tokens);

        let lp_tokens = &mut tokens[mkt.lp];
        lp_tokens.transaction();
        lp_tokens.add_balance(&from, to_mint);
        lp_tokens.end_transaction();
        tokens[id.0].end_transaction();
        tokens[id.1].end_transaction();
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
        tokens[id.0].transaction();
        tokens[id.1].transaction();
        // the zero is the one to be computed
        if !(amount_a == 0 || amount_b == 0) {
            return;
        }

        if !(tokens[id.0].balance_check(&from) >= amount_a
            && tokens[id.1].balance_check(&from) >= amount_b)
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

        tokens[id.0].balance_sub(&from, amount_a);
        tokens[id.1].balance_sub(&from, amount_b);

        if amount_a == 0 {
            let new_amount_a = (mkt.amt_a(tokens) * amount_b) / mkt.amt_b(tokens);
            tokens[id.0].add_balance(&from, new_amount_a);
            // modify market
            tokens[id.0].balance_sub(&mkt.id, new_amount_a);
            tokens[id.1].add_balance(&mkt.id, amount_b);
        } else {
            let new_amount_b = (mkt.amt_b(tokens) * amount_a) / mkt.amt_a(tokens);
            tokens[id.1].add_balance(&from, new_amount_b);
            // modify market
            tokens[id.1].balance_sub(&mkt.id, new_amount_b);
            tokens[id.0].add_balance(&mkt.id, amount_a);
        }

        tokens[id.0].end_transaction();
        tokens[id.1].end_transaction();
    }
}
