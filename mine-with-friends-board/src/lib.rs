use erc20::{ERC20Ptr, ERC20Registry, ERC20Standard};
use serde::{ser::SerializeSeq, Serialize};
use std::collections::btree_map::*;

mod erc20;
#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Serialize, Copy)]
enum UserID {
    Contract(u128),
}

mod token_swap;

pub struct ContractCreator(u128);
impl ContractCreator {
    pub(crate) fn make(&mut self) -> UserID {
        self.0 += 1;
        UserID::Contract(self.0)
    }
}

mod nft;

struct GameBoard {
    erc20s: erc20::ERC20Registry,
    swap: token_swap::Uniswap,
    /// Make this a vote over the map of users to current vote and let the turn count be dynamic
    turn_count: u64,
    alloc: ContractCreator,
    users: BTreeMap<UserID, String>,
    nfts: nft::NFTRegistry,
    nft_sales: nft::NFTSaleRegistry,
    power_plants: (),
    player_move_sequence: BTreeMap<UserID, u64>,
    new_users_allowed: bool,
    init: bool,
    /// If init = true, must be Some
    bitcoin_token_id: Option<ERC20Ptr>,
    /// If init = true, must be Some
    dollar_token_id: Option<ERC20Ptr>,

    root_user: Option<UserID>,
}
impl GameBoard {
    fn setup() -> GameBoard {
        todo!();
    }
    fn play(
        &mut self,
        Verified {
            d,
            sequence,
            sig,
            from,
        }: Verified<Unsanitized<GameMove>>,
    ) {
        // TODO: check that sequence is the next sequence for that particular user
        let current_move = self.player_move_sequence.entry(from.clone()).or_default();
        if (*current_move + 1) != sequence {
            return;
        } else {
            *current_move = sequence;
        }
        // TODO: verify the key/sig/d combo (or it happens during deserialization of Verified)
        match d.sanitize(()) {
            GameMove::Init => {
                if self.init == false {
                    self.init = true;
                    self.bitcoin_token_id
                        .insert(self.erc20s.new_token(Box::new(ERC20Standard::default())));
                    self.dollar_token_id
                        .insert(self.erc20s.new_token(Box::new(ERC20Standard::default())));
                    self.root_user.insert(self.alloc.make());
                    // TODO: Initialize Power Plants?
                    let demo_nft = self.nfts.add(Box::new(nft::BaseNFT {
                        owner: self.root_user.unwrap(),
                        plant_id: self.alloc.make(),
                        transfer_count: 0,
                    }));
                    self.nft_sales.list_nft(
                        demo_nft,
                        1000,
                        self.bitcoin_token_id.unwrap(),
                        &self.nfts,
                    );
                }
            }
            GameMove::RegisterUser(user) => {
                if self.new_users_allowed {
                    self.users.insert(self.alloc.make(), user);
                }
            }
            GameMove::NoNewUsers => {
                self.new_users_allowed = false;
            }
            GameMove::Trade(pair, a, b) => {
                self.swap
                    .do_trade(&mut self.erc20s, &mut self.alloc, pair, a, b, from);
            }
            GameMove::PurchaseNFT(asset, limit_price, currency) => self.nft_sales.make_trade(
                from,
                asset,
                &mut self.erc20s,
                &mut self.nfts,
                limit_price,
                currency,
            ),
            GameMove::ListNFTForSale(asset, price, currency) => {
                self.nft_sales.list_nft(asset, price, currency, &self.nfts)
            }
        }
    }
}

enum GameMove {
    Init,
    NoNewUsers,
    Trade(token_swap::PairID, u128, u128),
    PurchaseNFT(nft::NftPtr, nft::Price, nft::Currency),
    ListNFTForSale(nft::NftPtr, nft::Price, nft::Currency),
    RegisterUser(String),
}

impl Sanitizable for GameMove {
    type Output = Self;
    type Context = ();
    fn sanitize(self, context: ()) -> Self {
        todo!()
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
