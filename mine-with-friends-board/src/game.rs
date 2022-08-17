use std::collections::BTreeMap;

use crate::entity::EntityID;
use crate::entity::EntityIDAllocator;
use crate::sanitize::Sanitizable;
use crate::token_swap;
use crate::token_swap::Uniswap;

use self::game_move::GameMove;
use self::game_move::Init;
use self::game_move::ListNFTForSale;
use self::game_move::NoNewUsers;
use self::game_move::PurchaseNFT;
use self::game_move::RegisterUser;
use self::game_move::Trade;

use super::erc20;
use super::nft;
use crate::sanitize;

use erc20::ERC20Standard;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::Verified;

use erc20::ERC20Ptr;

use erc20::ERC20Registry;

#[derive(Serialize)]
pub struct GameBoard {
    pub(crate) erc20s: erc20::ERC20Registry,
    pub(crate) swap: token_swap::Uniswap,
    /// Make this a vote over the map of users to current vote and let the turn count be dynamic
    pub(crate) turn_count: u64,
    pub(crate) alloc: EntityIDAllocator,
    pub(crate) users: BTreeMap<EntityID, String>,
    pub(crate) nfts: nft::NFTRegistry,
    pub(crate) nft_sales: nft::NFTSaleRegistry,
    pub(crate) power_plants: (),
    pub(crate) player_move_sequence: BTreeMap<EntityID, u64>,
    pub(crate) new_users_allowed: bool,
    pub(crate) init: bool,
    /// If init = true, must be Some
    pub(crate) bitcoin_token_id: Option<ERC20Ptr>,
    /// If init = true, must be Some
    pub(crate) dollar_token_id: Option<ERC20Ptr>,

    pub(crate) root_user: Option<EntityID>,
}

impl GameBoard {
    pub fn new() -> GameBoard {
        GameBoard {
            erc20s: ERC20Registry::default(),
            swap: Default::default(),
            turn_count: 0,
            alloc: EntityIDAllocator(0x00C0DE0000),
            users: Default::default(),
            nfts: Default::default(),
            nft_sales: Default::default(),
            power_plants: (),
            player_move_sequence: Default::default(),
            new_users_allowed: true,
            init: false,
            bitcoin_token_id: None,
            dollar_token_id: None,
            root_user: None,
        }
    }
    pub fn play(
        &mut self,
        Verified {
            d,
            sequence,
            sig,
            from,
        }: Verified<sanitize::Unsanitized<GameMove>>,
    ) -> Result<(), ()> {
        // TODO: check that sequence is the next sequence for that particular user
        let current_move = self.player_move_sequence.entry(from.clone()).or_default();
        if (*current_move + 1) != sequence {
            return Ok(());
        } else {
            *current_move = sequence;
        }
        // TODO: verify the key/sig/d combo (or it happens during deserialization of Verified)
        match d.sanitize(())? {
            GameMove::Init(Init {}) => {
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
                        nft_id: self.alloc.make(),
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
            GameMove::RegisterUser(RegisterUser { user_id }) => {
                if self.new_users_allowed {
                    self.users.insert(self.alloc.make(), user_id);
                }
            }
            GameMove::NoNewUsers(NoNewUsers {}) => {
                self.new_users_allowed = false;
            }
            GameMove::Trade(Trade {
                pair,
                amount_a,
                amount_b,
            }) => {
                self.swap.do_trade(
                    &mut self.erc20s,
                    &mut self.alloc,
                    pair,
                    amount_a,
                    amount_b,
                    from,
                );
            }
            GameMove::PurchaseNFT(PurchaseNFT {
                nft_id,
                limit_price,
                currency,
            }) => self.nft_sales.make_trade(
                from,
                nft_id,
                &mut self.erc20s,
                &mut self.nfts,
                limit_price,
                currency,
            ),
            GameMove::ListNFTForSale(ListNFTForSale {
                nft_id,
                price,
                currency,
            }) => self.nft_sales.list_nft(nft_id, price, currency, &self.nfts),
        }
        return Ok(());
    }
}

pub mod game_move;
