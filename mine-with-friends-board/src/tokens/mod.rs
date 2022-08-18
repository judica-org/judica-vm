use super::entity::EntityID;
use crate::{callbacks::Callback, entity::EntityIDAllocator, nft::Price};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    cmp::min,
    collections::BTreeMap,
    ops::{Index, IndexMut},
};
use token_swap::{ConstantFunctionMarketMaker, TradingPairID};

pub mod token_swap;
pub(crate) trait Token: Send + Sync {
    fn transaction(&mut self);
    // todo: undo if transaction fails?
    fn end_transaction(&mut self);
    fn mint(&mut self, to: &EntityID, amount: u128);
    fn burn(&mut self, to: &EntityID, amount: u128);
    fn balance_check(&mut self, to: &EntityID) -> u128;
    #[must_use]
    fn transfer(&mut self, sender: &EntityID, receiver: &EntityID, amount: u128) -> bool;
    fn total_coins(&self) -> u128;
    fn to_json(&self) -> serde_json::Value;
    fn id(&self) -> EntityID;
    fn nickname(&self) -> Option<String>;
}

#[derive(Serialize)]
pub(crate) struct TokenBase {
    pub(crate) balances: BTreeMap<EntityID, u128>,
    pub(crate) total: u128,
    #[cfg(test)]
    pub(crate) in_transaction: Option<u128>,
    pub this: EntityID,
    pub nickname: Option<String>,
}

impl TokenBase {
    fn check_in_transaction(&self) {
        #[cfg(test)]
        if !self.in_transaction.is_some() {
            panic!("Not In Transaction Currently");
        }
    }
}
impl TokenBase {
    pub fn new(alloc: &mut EntityIDAllocator, nickname: String) -> Self {
        Self {
            balances: Default::default(),
            total: Default::default(),
            this: alloc.make(),
            #[cfg(test)]
            in_transaction: None,
            nickname: Some(nickname),
        }
    }
}
impl Token for TokenBase {
    fn mint(&mut self, to: &EntityID, amount: u128) {
        self.check_in_transaction();
        let amt = self.balances.entry(to.clone()).or_default();
        *amt += amount;
        self.total += amount;
    }
    fn burn(&mut self, to: &EntityID, amount: u128) {
        self.check_in_transaction();
        let amt = self.balances.entry(to.clone()).or_default();
        *amt -= amount;
        self.total -= amount;
    }

    fn balance_check(&mut self, to: &EntityID) -> u128 {
        *self.balances.entry(to.clone()).or_default()
    }
    fn total_coins(&self) -> u128 {
        self.total
    }
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(&self).unwrap()
    }

    fn transaction(&mut self) {
        #[cfg(test)]
        {
            if self.in_transaction.is_some() {
                panic!("Should Not Be Called, currently in transaction");
            } else {
                self.in_transaction = Some(self.total);
            }
        }
    }

    fn end_transaction(&mut self) {
        #[cfg(test)]
        {
            if self.in_transaction.is_none() {
                panic!("Should Not Be Called, was not in transaction");
            } else {
                if self.in_transaction != Some(self.total) {
                    panic!("Transaction did not preserve the coins")
                }
                self.in_transaction = None;
            }
        }
    }

    fn transfer(&mut self, sender: &EntityID, receiver: &EntityID, amount: u128) -> bool {
        if self.balance_check(sender) < amount {
            return false;
        }
        self.burn(sender, amount);
        self.mint(receiver, amount);
        return true;
    }

    fn id(&self) -> EntityID {
        self.this
    }

    fn nickname(&self) -> Option<String> {
        self.nickname.clone()
    }
}

#[derive(Deserialize, Serialize, Eq, Ord, PartialEq, PartialOrd, Copy, Clone, JsonSchema)]
#[serde(transparent)]
pub struct TokenPointer(EntityID);
impl TokenPointer {
    pub fn inner(&self) -> u64 {
        self.0 .0
    }
}

#[derive(Serialize)]
pub struct HashBoardData {
    pub hash_per_watt: u128,
    // out of 100, currently not used for anything.
    // Could be used to determine "burn out"
    pub reliability: u8,
}

#[derive(Clone)]
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

#[derive(Default, Serialize)]
pub(crate) struct TokenRegistry {
    #[serde(serialize_with = "special_serializer")]
    pub tokens: BTreeMap<EntityID, Box<dyn Token>>,
    pub hashboards: BTreeMap<TokenPointer, HashBoardData>,
}

pub(crate) fn special_serializer<S>(
    v: &BTreeMap<EntityID, Box<dyn Token>>,
    s: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.collect_map(v.iter().map(|b| (b.0, b.1.to_json())))
}

impl TokenRegistry {
    pub(crate) fn new_token(&mut self, new: Box<dyn Token>) -> TokenPointer {
        let p = TokenPointer(new.id());
        self.tokens.insert(new.id(), new);
        p
    }
}

impl Index<TokenPointer> for TokenRegistry {
    type Output = Box<dyn Token>;

    fn index(&self, index: TokenPointer) -> &Self::Output {
        self.tokens.get(&index.0).unwrap()
    }
}

impl IndexMut<TokenPointer> for TokenRegistry {
    fn index_mut(&mut self, index: TokenPointer) -> &mut Self::Output {
        self.tokens.get_mut(&index.0).unwrap()
    }
}
