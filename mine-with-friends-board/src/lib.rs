use erc20::{ERC20Ptr, ERC20Registry, ERC20Standard};
use serde::{ser::SerializeSeq, Serialize};
use std::{
    collections::btree_map::*,
    ops::{Index, IndexMut},
};

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

trait NFT {
    fn owner(&self) -> UserID;
    fn transfer(&mut self, to: UserID);
    fn id(&self) -> UserID;
    fn transfer_count(&self) -> u128;
}

type Price = u128;
type Currency = ERC20Ptr;
type NFTID = UserID;
struct NFTRegistry {
    nfts: BTreeMap<NftPtr, Box<dyn NFT>>,
}
#[derive(Serialize, Eq, Ord, PartialEq, PartialOrd, Clone, Copy)]
struct NftPtr(UserID);
impl NFTRegistry {
    fn add(&mut self, nft: Box<dyn NFT>) -> NftPtr {
        let id = NftPtr(nft.id());
        if self.nfts.contains_key(&id) {
        } else {
            self.nfts.insert(id.clone(), nft);
        }
        id
    }
}

impl Index<NftPtr> for NFTRegistry {
    type Output = Box<dyn NFT>;

    fn index(&self, index: NftPtr) -> &Self::Output {
        self.nfts.get(&index).unwrap()
    }
}
impl IndexMut<NftPtr> for NFTRegistry {
    fn index_mut(&mut self, index: NftPtr) -> &mut Self::Output {
        self.nfts.get_mut(&index).unwrap()
    }
}

struct NFTSaleRegistry {
    nfts: BTreeMap<NftPtr, (Price, Currency, UserID, u128)>,
}
impl NFTSaleRegistry {
    fn list_nft(&mut self, asset: NftPtr, price: Price, currency: Currency, nfts: &NFTRegistry) {
        self.nfts.insert(
            asset,
            (
                price,
                currency,
                nfts[asset].owner(),
                nfts[asset].transfer_count(),
            ),
        );
    }
    fn make_trade(
        &mut self,
        to: UserID,
        asset: NftPtr,
        tokens: &mut ERC20Registry,
        nfts: &mut NFTRegistry,
        limit_price: Price,
        limit_currency: Currency,
    ) {
        if let Some((price, currency, who, transfer_count)) = self.nfts.get(&asset) {
            if *who != nfts[asset.clone()].owner() {
                return;
            }
            if *transfer_count != nfts[asset.clone()].transfer_count() {
                return;
            }
            if limit_currency != *currency {
                return;
            }
            if limit_price >= *price {
                return;
            }
            let token = &mut tokens[currency.clone()];
            token.transaction();
            if token.transfer(&to, &nfts[asset.clone()].owner(), *price) {
                nfts[asset].transfer(to);
            }
            token.end_transaction();
        }
    }
}

struct BaseNFT {
    owner: UserID,
    plant_id: UserID,
    transfer_count: u128,
}

impl NFT for BaseNFT {
    fn owner(&self) -> UserID {
        self.owner
    }

    fn transfer(&mut self, to: UserID) {
        self.owner = to;
        self.transfer_count += 1;
    }

    fn id(&self) -> UserID {
        self.plant_id
    }

    fn transfer_count(&self) -> u128 {
        self.transfer_count
    }
}

struct PowerPlant {
    base: BaseNFT,
}

impl NFT for PowerPlant {
    fn owner(&self) -> UserID {
        self.base.owner()
    }

    fn transfer(&mut self, to: UserID) {
        self.base.transfer(to)
    }

    fn id(&self) -> UserID {
        self.base.id()
    }

    fn transfer_count(&self) -> u128 {
        self.base.transfer_count()
    }
}

struct GameBoard {
    erc20s: erc20::ERC20Registry,
    swap: token_swap::Uniswap,
    /// Make this a vote over the map of users to current vote and let the turn count be dynamic
    turn_count: u64,
    alloc: ContractCreator,
    users: BTreeMap<UserID, String>,
    nfts: NFTRegistry,
    nft_sales: NFTSaleRegistry,
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
                    let demo_nft = self.nfts.add(Box::new(BaseNFT {
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
    PurchaseNFT(NftPtr, Price, Currency),
    ListNFTForSale(NftPtr, Price, Currency),
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
