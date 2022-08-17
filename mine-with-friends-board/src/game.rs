use std::collections::BTreeMap;

use crate::token_swap;

use super::erc20;
use super::nft;
use super::Sanitizable;

use erc20::ERC20Standard;
use serde::Deserialize;
use serde::Serialize;

use super::Unsanitized;

use super::Verified;

use erc20::ERC20Ptr;

use super::UserID;

use super::ContractCreator;

use erc20::ERC20Registry;

pub(crate) struct GameBoard {
    pub(crate) erc20s: erc20::ERC20Registry,
    pub(crate) swap: token_swap::Uniswap,
    /// Make this a vote over the map of users to current vote and let the turn count be dynamic
    pub(crate) turn_count: u64,
    pub(crate) alloc: ContractCreator,
    pub(crate) users: BTreeMap<UserID, String>,
    pub(crate) nfts: nft::NFTRegistry,
    pub(crate) nft_sales: nft::NFTSaleRegistry,
    pub(crate) power_plants: (),
    pub(crate) player_move_sequence: BTreeMap<UserID, u64>,
    pub(crate) new_users_allowed: bool,
    pub(crate) init: bool,
    /// If init = true, must be Some
    pub(crate) bitcoin_token_id: Option<ERC20Ptr>,
    /// If init = true, must be Some
    pub(crate) dollar_token_id: Option<ERC20Ptr>,

    pub(crate) root_user: Option<UserID>,
}

impl GameBoard {
    pub(crate) fn setup() -> GameBoard {
        todo!();
    }
    pub(crate) fn play(
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

#[derive(Serialize, Deserialize)]
pub(crate) enum GameMove {
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
