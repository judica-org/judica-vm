use crate::nfts::instances::powerplant::PlantType;
use crate::nfts::NftPtr;
use crate::tokens::token_swap::TradingPairID;
use crate::util::Currency;
use crate::{entity::EntityID, util::Price};
use crate::{sanitize, MoveEnvelope};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Grab-Bag Enum of all moves
///
/// N.B. we do the enum-of-struct-variant pattern to make serialization/schemas
/// nicer.
#[derive(Debug, Serialize, Deserialize, JsonSchema, Eq, PartialEq, Clone)]
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
    /// # Remove Tokens
    RemoveTokens(RemoveTokens),
    /// # Send a logged Chat Message to All Players
    Chat(Chat),
    /// # Mint Power Plant NFT
    MintPowerPlant(MintPowerPlant),
    /// # Purchase Materials, then Mint Power Plant NFT
    SuperMintPowerPlant(MintPowerPlant),
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
derive_from!(RemoveTokens);
derive_from!(Chat);

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Heartbeat();

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Trade {
    pub pair: TradingPairID,
    pub amount_a: u128,
    pub amount_b: u128,
    pub sell: bool,
    #[serde(default)]
    pub cap: Option<u128>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema, Clone)]
pub struct MintPowerPlant {
    /// Size of the power plant
    pub scale: u64,
    pub location: (i64, i64),
    pub plant_type: PlantType,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema, Clone)]
pub struct PurchaseNFT {
    pub nft_id: NftPtr,
    pub limit_price: Price,
    pub currency: Currency,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema, Clone)]
pub struct ListNFTForSale {
    pub nft_id: NftPtr,
    pub price: Price,
    pub currency: Currency,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema, Clone)]
pub struct SendTokens {
    pub to: EntityID,
    pub amount: Price,
    pub currency: Currency,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema, Clone)]
pub struct RemoveTokens {
    pub nft_id: NftPtr,
    pub amount: Price,
    pub currency: Currency,
    // do we need time here?
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Chat(pub String);

impl MoveEnvelope {
    pub fn create<G: Into<GameMove>>(
        g: G,
        sequence: u64,
        _sig: String,
        _from: EntityID,
        time: u64,
    ) -> Self {
        MoveEnvelope {
            d: sanitize::Unsanitized(g.into()),
            sequence,
            time_millis: time,
        }
    }
}
