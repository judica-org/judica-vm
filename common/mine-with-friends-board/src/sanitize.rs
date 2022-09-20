//! This module helps with sanitizing certain things about different data types.
//!
//! TODO: The Context Objects passed in should e.g. be sufficient to check that all pointers are valid
//! Currently this is not done.
use serde::{Deserialize, Serialize};

use crate::{
    game::game_move::{
        Chat, GameMove, Heartbeat, ListNFTForSale, MintPowerPlant, PurchaseNFT, SendTokens, Trade,
    },
    nfts::NftPtr,
    tokens::token_swap::TradingPairID,
    tokens::TokenPointer,
};

pub trait Sanitizable {
    type Output;
    type Context;
    type Error;
    fn sanitize(self, context: Self::Context) -> Result<Self::Output, Self::Error>;
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct Unsanitized<D: Sanitizable>(pub D);

impl<D> Sanitizable for Unsanitized<D>
where
    D: Sanitizable,
{
    type Output = D::Output;
    type Context = D::Context;
    type Error = D::Error;
    fn sanitize(self, context: D::Context) -> Result<D::Output, D::Error> {
        self.0.sanitize(context)
    }
}

impl Sanitizable for TokenPointer {
    type Output = TokenPointer;
    type Context = ();
    type Error = ();
    fn sanitize(self, _context: Self::Context) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}
impl Sanitizable for NftPtr {
    type Output = NftPtr;
    type Context = ();
    type Error = ();
    fn sanitize(self, _context: Self::Context) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}

impl Sanitizable for TradingPairID {
    type Output = TradingPairID;
    type Context = <TokenPointer as Sanitizable>::Context;
    type Error = <TokenPointer as Sanitizable>::Error;
    fn sanitize(self, _context: Self::Context) -> Result<Self::Output, Self::Error> {
        let pair = TradingPairID {
            asset_a: self.asset_a.sanitize(())?,
            asset_b: self.asset_b.sanitize(())?,
        };
        Ok(pair)
    }
}

impl Sanitizable for GameMove {
    type Output = Self;
    type Context = ();
    type Error = ();
    fn sanitize(self, _context: ()) -> Result<Self, Self::Error> {
        Ok(match self {
            GameMove::Heartbeat(x) => x.sanitize(())?.into(),
            GameMove::Trade(x) => x.sanitize(())?.into(),
            GameMove::MintPowerPlant(x) => x.sanitize(())?.into(),
            GameMove::SuperMintPowerPlant(x) => x.sanitize(())?.into(),
            GameMove::PurchaseNFT(x) => x.sanitize(())?.into(),
            GameMove::ListNFTForSale(x) => x.sanitize(())?.into(),
            GameMove::SendTokens(x) => x.sanitize(())?.into(),
            GameMove::Chat(x) => x.sanitize(())?.into(),
        })
    }
}

impl Sanitizable for Heartbeat {
    type Output = Self;
    type Context = ();
    type Error = ();
    fn sanitize(self, _context: Self::Context) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}

impl Sanitizable for Trade {
    type Output = Self;
    type Context = ();
    type Error = ();
    fn sanitize(self, _context: Self::Context) -> Result<Self::Output, Self::Error> {
        let Self {
            pair,
            amount_a,
            amount_b,
        } = self;
        Ok(Self {
            pair: pair.sanitize(())?,
            amount_a,
            amount_b,
        })
    }
}

impl Sanitizable for Chat {
    type Output = Self;
    type Context = ();
    type Error = ();

    fn sanitize(self, _context: Self::Context) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}

// because resources is Vec<(TokenPointer, u128)> may need make Resources a type and sanitize.
impl Sanitizable for MintPowerPlant {
    type Output = Self;
    type Context = ();
    type Error = ();
    fn sanitize(self, _context: Self::Context) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}

impl Sanitizable for PurchaseNFT {
    type Output = Self;
    type Context = ();
    type Error = ();
    fn sanitize(self, _context: Self::Context) -> Result<Self::Output, Self::Error> {
        let Self {
            nft_id,
            limit_price,
            currency,
        } = self;
        Ok(Self {
            nft_id: nft_id.sanitize(())?,
            limit_price,
            currency: currency.sanitize(())?,
        })
    }
}
impl Sanitizable for ListNFTForSale {
    type Output = Self;
    type Context = ();
    type Error = ();
    fn sanitize(self, _context: Self::Context) -> Result<Self::Output, Self::Error> {
        let Self {
            nft_id,
            price,
            currency,
        } = self;
        Ok(Self {
            nft_id: nft_id.sanitize(())?,
            price,
            currency: currency.sanitize(())?,
        })
    }
}

impl Sanitizable for SendTokens {
    type Output = Self;
    type Context = ();
    type Error = ();
    fn sanitize(self, _context: Self::Context) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}
