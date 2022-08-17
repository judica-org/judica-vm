use erc20::ERC20Ptr;
use serde::{ser::SerializeSeq, Deserialize, Serialize};
use std::collections::btree_map::*;

mod erc20;
#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Serialize, Copy, Deserialize)]
enum UserID {
    Contract(u128),
}

mod game;
mod nft;
mod token_swap;

pub struct ContractCreator(u128);
impl ContractCreator {
    pub(crate) fn make(&mut self) -> UserID {
        self.0 += 1;
        UserID::Contract(self.0)
    }
}

trait Sanitizable {
    type Output;
    type Context;
    fn sanitize(self, context: Self::Context) -> Self::Output;
}
struct Unsanitized<D: Sanitizable>(D);
impl<D> Sanitizable for Unsanitized<D>
where
    D: Sanitizable,
{
    type Output = D::Output;
    type Context = D::Context;
    fn sanitize(self, context: D::Context) -> D::Output {
        self.0.sanitize(context)
    }
}
impl Sanitizable for ERC20Ptr {
    type Output = ERC20Ptr;
    type Context = ();
    fn sanitize(self, context: Self::Context) -> Self::Output {
        todo!()
    }
}

struct Verified<D> {
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
