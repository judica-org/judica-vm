use super::NFTRegistry;
use super::NftPtr;
use crate::entity::EntityID;
use crate::game::CallContext;
use crate::tokens::TokenRegistry;
use crate::util::Currency;
use crate::util::Price;
use serde::Serialize;
use std::collections::BTreeMap;

// Listings formatted for UX consumption
#[derive(Serialize, Clone)]
pub struct UXForSaleList {
    pub listings: Vec<UXNFTSale>,
}
#[derive(Serialize, Clone)]
pub struct UXNFTSale {
    pub nft_id: NftPtr,
    pub price: Price,
    pub currency: Currency,
    pub seller: EntityID,
    pub transfer_count: u128,
}
/// Represents an offer to sell an NFT
#[derive(Serialize)]
pub struct NFTSale {
    /// The Price the owner will accept
    pub price: Price,
    /// The Currency the owner will be paid in
    pub currency: Currency,
    /// The seller's ID _at the time the sale was opened_, for replay protection
    pub seller: EntityID,
    /// The transfer_count of the NFT _at the time the sale was opened_, for replay protection
    pub transfer_count: u128,
}
/// A Registry of all pending sales
#[derive(Serialize, Default)]
pub(crate) struct NFTSaleRegistry {
    pub(crate) nfts: BTreeMap<NftPtr, NFTSale>,
}

impl NFTSaleRegistry {
    /// Remove a sale of an NFT if the user is the owner
    pub fn cancel_sale(&mut self, asset: NftPtr, nfts: &NFTRegistry, user: EntityID) {
        if let Some(NFTSale { seller, .. }) = self.nfts.get(&asset) {
            if *seller == nfts[asset].owner() && *seller == user {
                self.nfts.remove(&asset);
            }
        }
    }
    /// List a sale of an NFT if the user is the owner
    pub(crate) fn list_nft(
        &mut self,
        CallContext { ref sender }: &CallContext,
        asset: NftPtr,
        price: Price,
        currency: Currency,
        nfts: &NFTRegistry,
    ) {
        if *sender == nfts[asset].owner() {
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
    }

    /// Execute the Purchase of an NFT
    pub(crate) fn purchase(
        &mut self,
        CallContext { ref sender }: &CallContext,
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
            if *seller != nfts[asset].owner() {
                return;
            }
            if *transfer_count != nfts[asset].transfer_count() {
                return;
            }
            if limit_currency != *currency {
                return;
            }
            if limit_price < *price {
                return;
            }
            let token = &mut tokens[(*currency)];
            token.transaction();
            if token.transfer(sender, &nfts[asset].owner(), *price) {
                // NOTE: transfer may fail, so revert if so.
                // Check is_transferable
                nfts[asset].transfer(*sender);
                self.nfts.remove(&asset);
            }
            token.end_transaction();
        }
    }
}
