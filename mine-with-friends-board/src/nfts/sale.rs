use super::NFTRegistry;
use super::NftPtr;
use crate::entity::EntityID;
use crate::tokens::TokenRegistry;
use crate::util::Currency;
use crate::util::Price;
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Serialize)]
pub struct NFTSale {
    price: Price,
    currency: Currency,
    seller: EntityID,
    transfer_count: u128,
}
#[derive(Serialize, Default)]
pub(crate) struct NFTSaleRegistry {
    pub(crate) nfts: BTreeMap<NftPtr, NFTSale>,
}

impl NFTSaleRegistry {
    pub(crate) fn list_nft(
        &mut self,
        asset: NftPtr,
        price: Price,
        currency: Currency,
        nfts: &NFTRegistry,
    ) {
        self.nfts.insert(
            asset,
            NFTSale {
                price,
                currency,
                seller: nfts[asset].owner(),
                transfer_count: nfts[asset].transfer_count(),
            },
        );
    }
    pub(crate) fn make_trade(
        &mut self,
        to: EntityID,
        asset: NftPtr,
        tokens: &mut TokenRegistry,
        nfts: &mut NFTRegistry,
        limit_price: Price,
        limit_currency: Currency,
    ) {
        if let Some(NFTSale {
            price,
            currency,
            seller,
            transfer_count,
        }) = self.nfts.get(&asset)
        {
            if *seller != nfts[asset.clone()].owner() {
                return;
            }
            if *transfer_count != nfts[asset.clone()].transfer_count() {
                return;
            }
            if limit_currency != *currency {
                return;
            }
            if limit_price < *price {
                return;
            }
            let token = &mut tokens[currency.clone()];
            token.transaction();
            if token.transfer(&to, &nfts[asset.clone()].owner(), *price) {
                /// NOTE: transfer may fail, so revert if so.
                /// Check is_transferable
                nfts[asset].transfer(to);
                self.nfts.remove(&asset);
            }
            token.end_transaction();
        }
    }
}
