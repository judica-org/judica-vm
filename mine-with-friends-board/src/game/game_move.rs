use crate::entity::EntityID;

use crate::sanitize;
use crate::sanitize::Unsanitized;
use crate::token_swap::TradingPairID;

use super::super::Verified;

use super::super::nft;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    pub nft_id: nft::NftPtr,
    pub limit_price: nft::Price,
    pub currency: nft::Currency,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ListNFTForSale {
    pub nft_id: nft::NftPtr,
    pub price: nft::Price,
    pub currency: nft::Currency,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct RegisterUser {
    pub user_id: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SendTokens {
    pub to: EntityID,
    pub amount: nft::Price,
    pub currency: nft::Currency,
}

impl Verified<Unsanitized<GameMove>> {
    pub fn create(g: GameMove, sequence: u64, sig: String, from: EntityID) -> Self {
        Verified {
            d: sanitize::Unsanitized(g),
            sequence,
            sig,
            from,
        }
    }
}
