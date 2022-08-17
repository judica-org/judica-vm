use crate::entity::EntityID;

use crate::sanitize;
use crate::sanitize::Unsanitized;
use crate::token_swap::PairID;

use super::super::Verified;

use super::super::nft;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GameMove {
    /// # Initialize Game
    Init(Init),
    /// # Stop Inviting New Users
    NoNewUsers(NoNewUsers),
    /// # Trade Coins
    Trade(Trade),
    /// # Buy NFTs
    PurchaseNFT(PurchaseNFT),
    /// # Sell NFTs
    ListNFTForSale(ListNFTForSale),
    /// # Register User
    RegisterUser(RegisterUser),
    /// # Send Coins
    SendTokens(SendTokens),
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
    pub pair: PairID,
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
