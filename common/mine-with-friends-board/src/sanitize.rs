// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! This module helps with sanitizing certain things about different data types.
//!
//! TODO: The Context Objects passed in should e.g. be sufficient to check that all pointers are valid
//! Currently this is not done.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    entity::EntityID,
    game::{
        game_move::{
            Chat, GameMove, Heartbeat, ListNFTForSale, MintPowerPlant, PurchaseNFT, RemoveTokens,
            SendTokens, Trade,
        },
        GameBoard,
    },
    nfts::NftPtr,
    tokens::token_swap::TradingPairID,
    tokens::TokenPointer,
};

#[derive(Serialize, Clone, Debug)]
pub enum SanitizationError {
    InvalidTokenPointer(TokenPointer),
    InvalidNFTPtr(NftPtr),
    InvalidUser(EntityID),
    MintScaleIsZero,
}

impl JsonSchema for SanitizationError {
    fn schema_name() -> String {
        "SanitizationError".into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        String::json_schema(gen)
    }
}
pub trait Sanitizable {
    type Output;
    type Context;
    type Error;
    fn sanitize(self, context: &Self::Context) -> Result<Self::Output, Self::Error>;
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, JsonSchema, Clone)]
pub struct Unsanitized<D: Sanitizable>(pub D);

impl<D> Sanitizable for Unsanitized<D>
where
    D: Sanitizable,
{
    type Output = D::Output;
    type Context = D::Context;
    type Error = D::Error;
    fn sanitize(self, context: &D::Context) -> Result<D::Output, D::Error> {
        self.0.sanitize(context)
    }
}

impl Sanitizable for TokenPointer {
    type Output = TokenPointer;
    type Context = GameBoard;
    type Error = SanitizationError;
    fn sanitize(self, context: &Self::Context) -> Result<Self::Output, Self::Error> {
        if context.tokens.tokens.contains_key(&self.as_id()) {
            Ok(self)
        } else {
            Err(SanitizationError::InvalidTokenPointer(self))
        }
    }
}
impl Sanitizable for NftPtr {
    type Output = NftPtr;
    type Context = GameBoard;
    type Error = SanitizationError;
    fn sanitize(self, context: &Self::Context) -> Result<Self::Output, Self::Error> {
        if context.nfts.nfts.contains_key(&self) {
            Ok(self)
        } else {
            Err(SanitizationError::InvalidNFTPtr(self))
        }
    }
}

impl Sanitizable for TradingPairID {
    type Output = TradingPairID;
    type Context = <TokenPointer as Sanitizable>::Context;
    type Error = <TokenPointer as Sanitizable>::Error;
    fn sanitize(self, context: &Self::Context) -> Result<Self::Output, Self::Error> {
        // No need to check normalization, since we may be using non-normal form
        // for trade direction
        let pair = TradingPairID {
            asset_a: self.asset_a.sanitize(context)?,
            asset_b: self.asset_b.sanitize(context)?,
        };
        Ok(pair)
    }
}

impl Sanitizable for GameMove {
    type Output = Self;
    type Context = GameBoard;
    type Error = SanitizationError;
    fn sanitize(self, context: &GameBoard) -> Result<Self, Self::Error> {
        Ok(match self {
            GameMove::RemoveTokens(x) => x.sanitize(context)?.into(),
            GameMove::Heartbeat(x) => x.sanitize(context)?.into(),
            GameMove::Trade(x) => x.sanitize(context)?.into(),
            GameMove::MintPowerPlant(x) => x.sanitize(context)?.into(),
            GameMove::SuperMintPowerPlant(x) => GameMove::SuperMintPowerPlant(x.sanitize(context)?),
            GameMove::PurchaseNFT(x) => x.sanitize(context)?.into(),
            GameMove::ListNFTForSale(x) => x.sanitize(context)?.into(),
            GameMove::SendTokens(x) => x.sanitize(context)?.into(),
            GameMove::Chat(x) => x.sanitize(context)?.into(),
        })
    }
}

impl Sanitizable for Heartbeat {
    type Output = Self;
    type Context = GameBoard;
    type Error = SanitizationError;
    fn sanitize(self, _context: &Self::Context) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}

impl Sanitizable for Trade {
    type Output = Self;
    type Context = GameBoard;
    type Error = SanitizationError;
    fn sanitize(self, context: &Self::Context) -> Result<Self::Output, Self::Error> {
        let Self {
            pair,
            amount_a,
            amount_b,
            sell,
            cap,
        } = self;
        // TODO: Sanitize amounts?
        Ok(Self {
            pair: pair.sanitize(context)?,
            amount_a,
            amount_b,
            sell,
            cap,
        })
    }
}

impl Sanitizable for Chat {
    type Output = Self;
    type Context = GameBoard;
    type Error = SanitizationError;

    fn sanitize(self, _context: &Self::Context) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}

// because resources is Vec<(TokenPointer, u128)> may need make Resources a type and sanitize.
impl Sanitizable for MintPowerPlant {
    type Output = Self;
    type Context = GameBoard;
    type Error = SanitizationError;
    fn sanitize(self, _context: &Self::Context) -> Result<Self::Output, Self::Error> {
        if self.scale == 0 {
            return Err(SanitizationError::MintScaleIsZero);
        }
        // TODO: CHeck that location is a valid coordinate
        Ok(self)
    }
}

impl Sanitizable for PurchaseNFT {
    type Output = Self;
    type Context = GameBoard;
    type Error = SanitizationError;
    fn sanitize(self, context: &Self::Context) -> Result<Self::Output, Self::Error> {
        let Self {
            nft_id,
            limit_price,
            currency,
        } = self;
        Ok(Self {
            nft_id: nft_id.sanitize(context)?,
            limit_price,
            currency: currency.sanitize(context)?,
        })
    }
}
impl Sanitizable for ListNFTForSale {
    type Output = Self;
    type Context = GameBoard;
    type Error = SanitizationError;
    fn sanitize(self, context: &Self::Context) -> Result<Self::Output, Self::Error> {
        let Self {
            nft_id,
            price,
            currency,
        } = self;
        Ok(Self {
            nft_id: nft_id.sanitize(context)?,
            price,
            currency: currency.sanitize(context)?,
        })
    }
}

impl Sanitizable for SendTokens {
    type Output = Self;
    type Context = GameBoard;
    type Error = SanitizationError;
    fn sanitize(self, context: &Self::Context) -> Result<Self::Output, Self::Error> {
        let SendTokens {
            to,
            amount,
            currency,
        } = self;
        if context.users.contains_key(&to) || context.nfts.nfts.contains_key(&to) {
            Ok(Self {
                to,
                amount,
                currency: currency.sanitize(context)?,
            })
        } else {
            Err(SanitizationError::InvalidUser(to))
        }
    }
}

impl Sanitizable for RemoveTokens {
    type Output = Self;
    type Context = GameBoard;
    type Error = SanitizationError;
    fn sanitize(self, context: &Self::Context) -> Result<Self::Output, Self::Error> {
        let Self {
            nft_id,
            amount,
            currency,
        } = self;
        Ok(Self {
            nft_id: nft_id.sanitize(context)?,
            amount,
            currency: currency.sanitize(context)?,
        })
    }
}
