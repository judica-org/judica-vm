use crate::{erc20::ERC20Ptr, game::GameMove, nft::NftPtr, token_swap::PairID};

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
    fn sanitize(self, context: Self::Context) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}
impl Sanitizable for NftPtr {
    type Output = NftPtr;
    type Context = ();
    type Error = ();
    fn sanitize(self, context: Self::Context) -> Result<Self::Output, Self::Error> {
        Ok(self)
    }
}
impl Sanitizable for PairID {
    type Output = PairID;
    type Context = <ERC20Ptr as Sanitizable>::Context;
    type Error = <ERC20Ptr as Sanitizable>::Error;
    fn sanitize(self, context: Self::Context) -> Result<Self::Output, Self::Error> {
        Ok(PairID(self.0.sanitize(())?, self.1.sanitize(())?))
    }
}

impl Sanitizable for GameMove {
    type Output = Self;
    type Context = ();
    type Error = ();
    fn sanitize(self, context: ()) -> Result<Self, Self::Error> {
        match self {
            GameMove::Init => Ok(self),
            GameMove::NoNewUsers => Ok(self),
            GameMove::Trade(a, b, c) => Ok(GameMove::Trade(a.sanitize(())?, b, c)),
            GameMove::PurchaseNFT(a, b, c) => {
                Ok(GameMove::PurchaseNFT(a.sanitize(())?, b, c.sanitize(())?))
            }
            GameMove::ListNFTForSale(a, b, c) => Ok(GameMove::ListNFTForSale(
                a.sanitize(())?,
                b,
                c.sanitize(())?,
            )),
            GameMove::RegisterUser(u) => Ok(GameMove::RegisterUser(u)),
        }
    }
}
