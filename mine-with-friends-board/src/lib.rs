use erc20::ERC20Ptr;
use serde::{ser::SerializeSeq, Deserialize, Serialize};
use std::{collections::btree_map::*, fmt::LowerHex, num::ParseIntError};

mod erc20;
#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Copy, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EntityID(pub u128);
impl TryFrom<String> for EntityID {
    type Error = ParseIntError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        u128::from_str_radix(&value, 16).map(EntityID)
    }
}
impl From<EntityID> for String {
    fn from(a: EntityID) -> Self {
        format!("{:x}", a.0)
    }
}

pub mod game;
mod nft;
pub mod sanitize;
mod token_swap;

#[derive(Serialize)]
pub struct ContractCreator(u128);
impl ContractCreator {
    pub(crate) fn make(&mut self) -> EntityID {
        self.0 += 1;
        EntityID(self.0)
    }
}

pub struct Verified<D> {
    d: D,
    sequence: u64,
    sig: String,
    from: EntityID,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
