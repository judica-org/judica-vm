//! This module defines components for managing and issuing tokens
use self::instances::{asics::HashBoardData, silicon::Silicon, steel::Steel};
use super::entity::EntityID;
use crate::{entity::EntityIDAllocator, game::GameBoard, util::Price};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ops::{Index, IndexMut},
};

pub mod instances;
pub mod token_swap;
/// Main Token trait
///
/// atts:
///     - transaction_required => a pairing of `transaction` and
///      `end_transaction` is required to wrap this call. May panic if not
///      (especially in test mode)
pub(crate) trait Token: Send + Sync {
    /// To Be called before calling any methods marked "transaction_required"
    fn transaction(&mut self);
    /// To Be called after calling any methods marked "transaction_required"
    /// *Does not undo if transaction fails*
    fn end_transaction(&mut self);

    /// Creates new coins and gives them to the `to` entity
    /// attr: transaction_required
    fn mint(&mut self, to: &EntityID, amount: Price);

    /// Burns an amount of coins held by the `to` entity
    /// attr: transaction_required
    fn burn(&mut self, to: &EntityID, amount: Price);

    /// Checks how much funds `to` has
    fn balance_check(&self, to: &EntityID) -> u128;
    /// Checks the total amount of coins
    fn total_coins(&self) -> u128;
    /// Transfer coins from the `sender` to the `receiver`.
    /// postcondition: if returns false, no effect. if true, transfer success.
    /// attr: transaction_required
    #[must_use]
    fn transfer(&mut self, sender: &EntityID, receiver: &EntityID, amount: Price) -> bool;
    /// gets a JSON representation of the token
    /// TODO: Standardize the repr?
    fn to_json(&self) -> serde_json::Value;
    /// the id of the token contract itself
    fn id(&self) -> EntityID;
    /// a nickname, not guaranteed to be unique
    fn nickname(&self) -> Option<String>;
}

/// A Basic Token Implementation
#[derive(Serialize)]
pub(crate) struct TokenBase {
    pub(crate) balances: BTreeMap<EntityID, Price>,
    /// Cached from the sum(balances.values())
    pub(crate) total: Price,
    #[cfg(test)]
    /// Test Only: if in a transaction, record the total amount before/after
    pub(crate) in_transaction: Option<Price>,
    /// this contract's ID
    pub this: EntityID,
    pub nickname: Option<String>,
}

impl TokenBase {
    /// function that only has an impact in test mode
    fn check_in_transaction(&self) {
        #[cfg(test)]
        if !self.in_transaction.is_some() {
            panic!("Not In Transaction Currently");
        }
    }
}
impl TokenBase {
    pub fn new_from_alloc(allocator: &mut EntityIDAllocator, nickname: String) -> Self {
        Self {
            balances: Default::default(),
            total: Default::default(),
            this: allocator.make(),
            #[cfg(test)]
            in_transaction: None,
            nickname: Some(nickname),
        }
    }
    /// Create a new token
    pub fn new(game: &mut GameBoard, nickname: String) -> Self {
        Self {
            balances: Default::default(),
            total: Default::default(),
            this: game.alloc(),
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

    fn balance_check(&self, to: &EntityID) -> u128 {
        self.balances.get(to).map_or(0, |x| x.clone())
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

/// TokenPointer helps to create a partially "Sanitized" pointer If we see a
/// TokenPointer, we know that it can be used to Index the TokenRegistry it came
/// from safely, or if it has been sanitized.
///
/// However, it cannot (currently) guarantee that it is from the correct source.
#[derive(
    Deserialize, Serialize, Eq, Ord, PartialEq, PartialOrd, Copy, Clone, JsonSchema, Debug,
)]
#[serde(transparent)]
pub struct TokenPointer(EntityID);
impl TokenPointer {
    pub fn inner(&self) -> u64 {
        self.0 .0
    }
}

/// Holds Tokens and metadata for custom token types
#[derive(Default, Serialize)]
pub(crate) struct TokenRegistry {
    #[serde(serialize_with = "special_serializer")]
    pub tokens: BTreeMap<EntityID, Box<dyn Token>>,
    pub hashboards: BTreeMap<TokenPointer, HashBoardData>,
    pub steel: BTreeMap<TokenPointer, Steel>,
    pub silicon: BTreeMap<TokenPointer, Silicon>,
}

/// Creates a readable form of a token
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
    /// Adds a token to our system
    /// N.B. does not ensure other subsystems are initiailized
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
