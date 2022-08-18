use std::collections::BTreeMap;

use crate::callbacks::CallbackRegistry;
use crate::entity::EntityID;
use crate::entity::EntityIDAllocator;

use crate::tokens::ASICProducer;
use crate::tokens::HashBoardData;
use crate::nft::PowerPlantEvent;
use crate::sanitize::Sanitizable;
use crate::token_swap;
use crate::token_swap::ConstantFunctionMarketMaker;

use self::game_move::GameMove;
use self::game_move::Init;
use self::game_move::ListNFTForSale;
use self::game_move::NoNewUsers;
use self::game_move::PurchaseNFT;
use self::game_move::RegisterUser;
use self::game_move::SendTokens;
use self::game_move::Trade;

use super::tokens;
use super::nft;
use crate::sanitize;

use tokens::TokenBase;

use serde::Serialize;

use super::Verified;

use tokens::TokenPointer;

use tokens::TokenRegistry;

#[derive(Serialize)]
pub struct GameBoard {
    pub(crate) tokens: tokens::TokenRegistry,
    pub(crate) swap: token_swap::ConstantFunctionMarketMaker,
    /// Make this a vote over the map of users to current vote and let the turn count be dynamic
    pub(crate) turn_count: u64,
    pub(crate) alloc: EntityIDAllocator,
    pub(crate) users: BTreeMap<EntityID, String>,
    pub(crate) nfts: nft::NFTRegistry,
    pub(crate) nft_sales: nft::NFTSaleRegistry,
    pub(crate) player_move_sequence: BTreeMap<EntityID, u64>,
    pub(crate) new_users_allowed: bool,
    pub(crate) init: bool,
    /// If init = true, must be Some
    pub(crate) bitcoin_token_id: Option<TokenPointer>,
    /// If init = true, must be Some
    pub(crate) dollar_token_id: Option<TokenPointer>,

    /// If init = true, must be Some
    pub(crate) root_user: Option<EntityID>,
    pub(crate) callbacks: CallbackRegistry,
    pub(crate) current_time: u64,
    pub(crate) mining_subsidy: u128,
}

impl GameBoard {
    pub fn new() -> GameBoard {
        GameBoard {
            tokens: TokenRegistry::default(),
            swap: Default::default(),
            turn_count: 0,
            alloc: EntityIDAllocator(0x00C0DE0000),
            users: Default::default(),
            nfts: Default::default(),
            nft_sales: Default::default(),
            player_move_sequence: Default::default(),
            new_users_allowed: true,
            init: false,
            bitcoin_token_id: None,
            dollar_token_id: None,
            root_user: None,
            callbacks: Default::default(),
            current_time: 0,
            mining_subsidy: 100_000_000_000 * 50,
        }
    }
    pub fn alloc(&mut self) -> EntityID {
        self.alloc.make()
    }
    pub fn root_user(&self) -> Option<EntityID> {
        self.root_user
    }
    pub fn user_is_admin(&self, user: EntityID) -> bool {
        Some(user) == self.root_user
    }

    pub fn play(
        &mut self,
        Verified {
            d,
            sequence,
            sig: _,
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
        let mv = d.sanitize(())?;
        if !self.user_is_admin(from) && mv.is_priviledged() {
            return Ok(());
        }
        // TODO: verify the key/sig/d combo (or it happens during deserialization of Verified)
        self.play_inner(mv, from)
    }
    pub fn play_inner(&mut self, d: GameMove, from: EntityID) -> Result<(), ()> {
        // TODO: verify the key/sig/d combo (or it happens during deserialization of Verified)
        match d {
            GameMove::Init(Init {}) => {
                if self.init == false {
                    self.init = true;
                    let _ = self.bitcoin_token_id.insert(self.tokens.new_token(Box::new(
                        TokenBase::new(&mut self.alloc, "Bitcoin".into()),
                    )));
                    let _ = self.dollar_token_id.insert(self.tokens.new_token(Box::new(
                        TokenBase::new(&mut self.alloc, "US Dollars".into()),
                    )));

                    let asic = self.tokens.new_token(Box::new(TokenBase::new(
                        &mut self.alloc,
                        "US Dollars".into(),
                    )));
                    let _ = self.tokens.hashboards.insert(
                        asic,
                        HashBoardData {
                            hash_per_watt: (3.0 * 10e12) as u128,
                            reliability: 100,
                        },
                    );
                    self.callbacks.schedule(Box::new(ASICProducer {
                        id: self.alloc.make(),
                        total_units: 100_000,
                        base_price: 20,
                        price_asset: self.dollar_token_id.unwrap(),
                        hash_asset: asic,
                        adjusts_every: 100, // what units?
                        current_time: self.current_time,
                        first: true,
                    }));
                    let root = self.alloc.make();
                    let _ = self.root_user.insert(root);

                    // DEMO CODE:
                    // REMOVE BEFORE FLIGHT
                    self.tokens[self.bitcoin_token_id.unwrap()].mint(&root, 10000000);
                    self.tokens[self.dollar_token_id.unwrap()].mint(&root, 30000);
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

                    self.callbacks.schedule(Box::new(PowerPlantEvent {
                        time: self.current_time + 100,
                        period: 100,
                    }));
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
                ConstantFunctionMarketMaker::do_trade(self, pair, amount_a, amount_b, from);
            }
            GameMove::PurchaseNFT(PurchaseNFT {
                nft_id,
                limit_price,
                currency,
            }) => self.nft_sales.make_trade(
                from,
                nft_id,
                &mut self.tokens,
                &mut self.nfts,
                limit_price,
                currency,
            ),
            GameMove::ListNFTForSale(ListNFTForSale {
                nft_id,
                price,
                currency,
            }) => self.nft_sales.list_nft(nft_id, price, currency, &self.nfts),
            GameMove::SendTokens(SendTokens {
                to,
                amount,
                currency,
            }) => {
                self.tokens[currency].transaction();
                self.tokens[currency].transfer(&from, &to, amount);
                self.tokens[currency].end_transaction();
            }
        }
        return Ok(());
    }
}

pub mod game_move;
