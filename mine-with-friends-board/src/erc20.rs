use super::entity::EntityID;
use schemars::JsonSchema;
use serde::{ser::SerializeSeq, Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ops::{Index, IndexMut},
};

pub(crate) trait ERC20: Send + Sync {
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
}

#[derive(Default, Serialize)]
pub(crate) struct ERC20Standard {
    pub(crate) balances: BTreeMap<EntityID, u128>,
    pub(crate) total: u128,
    #[cfg(test)]
    pub(crate) in_transaction: Option<u128>,
}

impl ERC20Standard {
    fn check_in_transaction(&self) {
        #[cfg(test)]
        if !self.in_transaction.is_some() {
            panic!("Not In Transaction Currently");
        }
    }
}
impl ERC20 for ERC20Standard {
    fn mint(&mut self, to: &EntityID, amount: u128) {
        self.check_in_transaction();
        let amt = self.balances.entry(to.clone()).or_default();
        *amt += amount;
        self.total += amount;
    }
    fn burn(&mut self, to: &EntityID, amount: u128) {
        self.check_in_transaction();
        let amt = self.balances.entry(to.clone()).or_default();
        *amt += amount;
        self.total += amount;
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
            if (self.in_transaction.is_some()) {
                panic!("Should Not Be Called, currently in transaction");
            } else {
                self.in_transaction = Some(self.total);
            }
        }
    }

    fn end_transaction(&mut self) {
        #[cfg(test)]
        {
            if (self.in_transaction.is_none()) {
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
}

#[derive(
    Default, Deserialize, Serialize, Eq, Ord, PartialEq, PartialOrd, Copy, Clone, JsonSchema,
)]
pub struct ERC20Ptr(usize);

#[derive(Default, Serialize)]
pub(crate) struct ERC20Registry(#[serde(serialize_with = "special_erc20")] Vec<Box<dyn ERC20>>);

pub(crate) fn special_erc20<S>(v: &Vec<Box<dyn ERC20>>, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut seq = s.serialize_seq(Some(v.len()))?;
    for elt in v.iter().map(|b| b.to_json()) {
        seq.serialize_element(&elt)?;
    }
    seq.end()
}

impl ERC20Registry {
    pub(crate) fn new_token(&mut self, new: Box<dyn ERC20>) -> ERC20Ptr {
        self.0.push(new);
        ERC20Ptr(self.0.len() - 1)
    }
}

impl Index<ERC20Ptr> for ERC20Registry {
    type Output = Box<dyn ERC20>;

    fn index(&self, index: ERC20Ptr) -> &Self::Output {
        &self.0[index.0]
    }
}

impl IndexMut<ERC20Ptr> for ERC20Registry {
    fn index_mut(&mut self, index: ERC20Ptr) -> &mut Self::Output {
        &mut self.0[index.0]
    }
}
