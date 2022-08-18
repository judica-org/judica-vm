use crate::{
    erc20::ERC20Ptr,
    game::game_move::{
        GameMove, Init, ListNFTForSale, NoNewUsers, PurchaseNFT, RegisterUser, SendTokens, Trade,
    },
    nft::NftPtr,
    token_swap::PairID,
};

pub trait Sanitizable {
    type Output;
    type Context;
    type Error;
    fn sanitize(self, context: Self::Context) -> Result<Self::Output, Self::Error>;
}

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

impl Sanitizable for ERC20Ptr {
    type Output = ERC20Ptr;
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
impl Sanitizable for PairID {
    type Output = PairID;
    type Context = <ERC20Ptr as Sanitizable>::Context;
    type Error = <ERC20Ptr as Sanitizable>::Error;
    fn sanitize(self, _context: Self::Context) -> Result<Self::Output, Self::Error> {
        let pair = PairID {
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
            GameMove::Init(x) => x.sanitize(())?.into(),
            GameMove::NoNewUsers(x) => x.sanitize(())?.into(),
            GameMove::Trade(x) => x.sanitize(())?.into(),
            GameMove::PurchaseNFT(x) => x.sanitize(())?.into(),
            GameMove::ListNFTForSale(x) => x.sanitize(())?.into(),
            GameMove::RegisterUser(x) => x.sanitize(())?.into(),
            GameMove::SendTokens(x) => x.sanitize(())?.into(),
        })
    }
}

impl Sanitizable for Init {
    type Output = Self;
    type Context = ();
    type Error = ();
    fn sanitize(self, _context: Self::Context) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}

impl Sanitizable for NoNewUsers {
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
impl Sanitizable for RegisterUser {
    type Output = Self;
    type Context = ();
    type Error = ();
    fn sanitize(self, _context: Self::Context) -> Result<Self::Output, Self::Error> {
        Ok(self)
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
