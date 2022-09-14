<<<<<<< HEAD
use crate::nfts::instances::powerplant::PlantType;
use crate::nfts::NftPtr;
=======
>>>>>>> af10827 (formatting fixes)
use crate::nfts::instances::powerplant::PlantType;
use crate::nfts::NftPtr;
use crate::util::Currency;
use crate::{entity::EntityID, util::Price};

use crate::sanitize;

use crate::tokens::token_swap::TradingPairID;

use super::super::MoveEnvelope;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Grab-Bag Enum of all moves
///
/// N.B. we do the enum-of-struct-variant pattern to make serialization/schemas
/// nicer.
#[derive(Debug, Serialize, Deserialize, JsonSchema, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GameMove {
    Heartbeat(Heartbeat),
    /// # Trade Coins
    Trade(Trade),
    /// # Buy NFTs
    PurchaseNFT(PurchaseNFT),
    /// # Sell NFTs
    ListNFTForSale(ListNFTForSale),
    /// # Send Coins
    SendTokens(SendTokens),
<<<<<<< HEAD
    /// # Send a logged Chat Message to All Players
    Chat(Chat),
    /// # Mint Power Plant NFT
    MintPowerPlant(MintPowerPlant),
=======
    /// # Mint NFT
    MintPowerPlant(MintPowerPlant),
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
            | GameMove::MintPowerPlant(_)
            | GameMove::PurchaseNFT(_)
            | GameMove::ListNFTForSale(_)
            | GameMove::SendTokens(_) => false,
            GameMove::Init(_) | GameMove::RegisterUser(_) | GameMove::NoNewUsers(_) => true,
        }
    }
>>>>>>> af10827 (formatting fixes)
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
derive_from!(Heartbeat);
derive_from!(Trade);
derive_from!(MintPowerPlant);
derive_from!(PurchaseNFT);
derive_from!(ListNFTForSale);
derive_from!(SendTokens);
derive_from!(Chat);

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Heartbeat();

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Trade {
    pub pair: TradingPairID,
    pub amount_a: u128,
    pub amount_b: u128,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MintPowerPlant {
    /// The token(s) and quantities required to mint the NFT
    pub resources: Vec<(Currency, Price)>,
    pub location: (u64, u64),
    pub plant_type: PlantType,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PurchaseNFT {
    pub nft_id: NftPtr,
    pub limit_price: Price,
    pub currency: Currency,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListNFTForSale {
    pub nft_id: NftPtr,
    pub price: Price,
    pub currency: Currency,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SendTokens {
    pub to: EntityID,
    pub amount: Price,
    pub currency: Currency,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Chat(pub String);

impl MoveEnvelope {
    pub fn create<G: Into<GameMove>>(
        g: G,
        sequence: u64,
        _sig: String,
        from: EntityID,
        time: u64,
    ) -> Self {
        MoveEnvelope {
            d: sanitize::Unsanitized(g.into()),
            sequence,
            from,
            time,
        }
    }
}
