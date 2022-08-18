use self::game_move::GameMove;
use self::game_move::Init;
use self::game_move::ListNFTForSale;
use self::game_move::NoNewUsers;
use self::game_move::PurchaseNFT;
use self::game_move::RegisterUser;
use self::game_move::SendTokens;
use self::game_move::Trade;
use crate::callbacks::CallbackRegistry;
use crate::entity::EntityID;
use crate::entity::EntityIDAllocator;
use crate::nfts::instances::powerplant::events::PowerPlantEvent;
use crate::nfts::sale::NFTSaleRegistry;
use crate::nfts::BaseNFT;
use crate::nfts::NFTRegistry;
use crate::sanitize;
use crate::sanitize::Sanitizable;
use crate::tokens;
use crate::tokens::instances::asics::ASICProducer;
use crate::tokens::instances::asics::HashBoardData;
use crate::tokens::token_swap;
use crate::tokens::token_swap::ConstantFunctionMarketMaker;
use crate::Verified;
use serde::Serialize;
use std::collections::BTreeMap;
use tokens::TokenBase;
use tokens::TokenPointer;
use tokens::TokenRegistry;

/// GameBoard holds the entire state of the game.
#[derive(Serialize)]
pub struct GameBoard {
    pub(crate) tokens: tokens::TokenRegistry,
    pub(crate) swap: token_swap::ConstantFunctionMarketMaker,
    /// Make this a vote over the map of users to current vote and let the turn count be dynamic
    pub(crate) turn_count: u64,
    alloc: EntityIDAllocator,
    pub(crate) users: BTreeMap<EntityID, String>,
    pub(crate) nfts: NFTRegistry,
    pub(crate) nft_sales: NFTSaleRegistry,
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

pub struct CallContext {
    pub sender: EntityID,
}

impl GameBoard {
    /// Creates a new GameBoard
    pub fn new() -> GameBoard {
        GameBoard {
            tokens: TokenRegistry::default(),
            swap: Default::default(),
            turn_count: 0,
            alloc: EntityIDAllocator::new(),
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
    /// Creates a new EntityID
    pub fn alloc(&mut self) -> EntityID {
        self.alloc.make()
    }
    pub fn root_user(&self) -> Option<EntityID> {
        self.root_user
    }
    /// Check if a given user is the root_user
    pub fn user_is_admin(&self, user: EntityID) -> bool {
        Some(user) == self.root_user
    }

    /// Processes a GameMove against the board after verifying it's integrity
    /// and sanitizing it.
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

    /// Processes a GameMove without any sanitization
    pub fn play_inner(&mut self, d: GameMove, from: EntityID) -> Result<(), ()> {
        // TODO: verify the key/sig/d combo (or it happens during deserialization of Verified)
        let context = CallContext { sender: from };
        match d {
            GameMove::Init(Init {}) => {
                if self.init == false {
                    self.init = true;
                    let btc = Box::new(TokenBase::new(self, "Bitcoin".into()));
                    let dollar = Box::new(TokenBase::new(self, "US Dollar".into()));
                    let asic = Box::new(TokenBase::new(self, "ASIC Gen 1".into()));
                    let _ = self.bitcoin_token_id.insert(self.tokens.new_token(btc));
                    let _ = self.dollar_token_id.insert(self.tokens.new_token(dollar));

                    let asic = self.tokens.new_token(asic);
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
                    let demo_nft = self.nfts.add(Box::new(BaseNFT {
                        owner: self.root_user.unwrap(),
                        nft_id: self.alloc.make(),
                        transfer_count: 0,
                    }));
                    self.nft_sales.list_nft(
                        &CallContext {
                            sender: self.root_user().unwrap(),
                        },
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
                ConstantFunctionMarketMaker::do_trade(self, pair, amount_a, amount_b, &context);
            }
            GameMove::PurchaseNFT(PurchaseNFT {
                nft_id,
                limit_price,
                currency,
            }) => self.nft_sales.purchase(
                &context,
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
            }) => self
                .nft_sales
                .list_nft(&context, nft_id, price, currency, &self.nfts),
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
