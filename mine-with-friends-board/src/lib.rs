use erc20::ERC20Ptr;
use serde::{ser::SerializeSeq, Deserialize, Serialize};
use std::collections::btree_map::*;

mod erc20;
#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Serialize, Copy, Deserialize)]
pub struct UserID(pub u128);

pub mod game;
mod nft;
mod token_swap;
pub mod sanitize;

#[derive(Serialize)]
pub struct ContractCreator(u128);
impl ContractCreator {
    pub(crate) fn make(&mut self) -> UserID {
        self.0 += 1;
        UserID(self.0)
    }
}


pub struct Verified<D> {
    d: D,
    sequence: u64,
    sig: String,
    from: UserID,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
