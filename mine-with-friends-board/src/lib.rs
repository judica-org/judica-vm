use erc20::ERC20Ptr;
use serde::{ser::SerializeSeq, Deserialize, Serialize};
use std::{collections::btree_map::*, fmt::LowerHex, num::ParseIntError};
pub mod entity;
pub mod erc20;
pub mod game;
pub mod nft;
pub mod sanitize;
pub mod token_swap;
mod callbacks;

pub struct Verified<D> {
    d: D,
    sequence: u64,
    sig: String,
    from: entity::EntityID,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
