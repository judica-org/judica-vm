use crate::nfts::NftPtr;
use crate::util::Currency;
use crate::{entity::EntityID, util::Price};

use crate::sanitize;
use crate::sanitize::Unsanitized;
use crate::tokens::token_swap::TradingPairID;

use super::super::MoveEnvelope;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Grab-Bag Enum of all moves
///
/// N.B. we do the enum-of-struct-variant pattern to make serialization/schemas
/// nicer.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GameMove {
    /// # Initialize Game
    #[schemars(skip)]
    Init(Init),
    /// # Stop Inviting New Users
    #[schemars(skip)]
    NoNewUsers(NoNewUsers),
    /// # Trade Coins
    Trade(Trade),
    /// # Buy NFTs
    PurchaseNFT(PurchaseNFT),
    /// # Sell NFTs
    ListNFTForSale(ListNFTForSale),
    /// # Register User
    #[schemars(skip)]
    RegisterUser(RegisterUser),
    /// # Send Coins
    SendTokens(SendTokens),
}

impl GameMove {
    /// These moves should only be made by the root user / system if true.
    /// TODO: Maybe have 3 rings for:
    /// - 0 system
    /// - 1 host
    /// - 2 player
    pub fn is_priviledged(&self) -> bool {
        match self {
            GameMove::Trade(_)
            | GameMove::PurchaseNFT(_)
            | GameMove::ListNFTForSale(_)
            | GameMove::SendTokens(_) => false,
            GameMove::Init(_) | GameMove::RegisterUser(_) | GameMove::NoNewUsers(_) => true,
        }
    }
}

// Convenience to marshall a move into a GameMove
macro_rules! derive_from {
    ($y:ident) => {
        impl From<$y> for GameMove {
            fn from(i: $y) -> Self {
                GameMove::$y(i)
            }
        }
    };
}
derive_from!(Init);
derive_from!(NoNewUsers);
derive_from!(Trade);
derive_from!(PurchaseNFT);
derive_from!(ListNFTForSale);
derive_from!(RegisterUser);
derive_from!(SendTokens);

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Init();
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct NoNewUsers();
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Trade {
    pub pair: TradingPairID,
    pub amount_a: u128,
    pub amount_b: u128,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct PurchaseNFT {
    pub nft_id: NftPtr,
    pub limit_price: Price,
    pub currency: Currency,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ListNFTForSale {
    pub nft_id: NftPtr,
    pub price: Price,
    pub currency: Currency,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct RegisterUser {
    pub user_id: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SendTokens {
    pub to: EntityID,
    pub amount: Price,
    pub currency: Currency,
}

impl MoveEnvelope {
    pub fn create(g: GameMove, sequence: u64, sig: String, from: EntityID, time: u64) -> Self {
        MoveEnvelope {
            d: sanitize::Unsanitized(g),
            sequence,
            sig,
            from,
            time,
        }
    }
}
