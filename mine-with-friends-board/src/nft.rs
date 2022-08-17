use crate::erc20::ERC20Registry;

use std::ops::IndexMut;

use std::ops::Index;

use crate::erc20::ERC20Ptr;

use super::UserID;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

pub(crate) trait NFT {
    fn owner(&self) -> UserID;
    fn transfer(&mut self, to: UserID);
    fn id(&self) -> UserID;
    fn transfer_count(&self) -> u128;
}

pub(crate) type Price = u128;

pub(crate) type Currency = ERC20Ptr;

pub(crate) type NFTID = UserID;

pub(crate) struct NFTRegistry {
    pub(crate) nfts: BTreeMap<NftPtr, Box<dyn NFT>>,
}

#[derive(Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd, Clone, Copy)]
pub(crate) struct NftPtr(UserID);

impl NFTRegistry {
    pub(crate) fn add(&mut self, nft: Box<dyn NFT>) -> NftPtr {
        let id = NftPtr(nft.id());
        if self.nfts.contains_key(&id) {
        } else {
            self.nfts.insert(id.clone(), nft);
        }
        id
    }
}

impl Index<NftPtr> for NFTRegistry {
    type Output = Box<dyn NFT>;

    fn index(&self, index: NftPtr) -> &Self::Output {
        self.nfts.get(&index).unwrap()
    }
}

impl IndexMut<NftPtr> for NFTRegistry {
    fn index_mut(&mut self, index: NftPtr) -> &mut Self::Output {
        self.nfts.get_mut(&index).unwrap()
    }
}

pub(crate) struct NFTSaleRegistry {
    pub(crate) nfts: BTreeMap<NftPtr, (Price, Currency, UserID, u128)>,
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
            (
                price,
                currency,
                nfts[asset].owner(),
                nfts[asset].transfer_count(),
            ),
        );
    }
    pub(crate) fn make_trade(
        &mut self,
        to: UserID,
        asset: NftPtr,
        tokens: &mut ERC20Registry,
        nfts: &mut NFTRegistry,
        limit_price: Price,
        limit_currency: Currency,
    ) {
        if let Some((price, currency, who, transfer_count)) = self.nfts.get(&asset) {
            if *who != nfts[asset.clone()].owner() {
                return;
            }
            if *transfer_count != nfts[asset.clone()].transfer_count() {
                return;
            }
            if limit_currency != *currency {
                return;
            }
            if limit_price >= *price {
                return;
            }
            let token = &mut tokens[currency.clone()];
            token.transaction();
            if token.transfer(&to, &nfts[asset.clone()].owner(), *price) {
                nfts[asset].transfer(to);
            }
            token.end_transaction();
        }
    }
}

pub(crate) struct BaseNFT {
    pub(crate) owner: UserID,
    pub(crate) plant_id: UserID,
    pub(crate) transfer_count: u128,
}

impl NFT for BaseNFT {
    fn owner(&self) -> UserID {
        self.owner
    }

    fn transfer(&mut self, to: UserID) {
        self.owner = to;
        self.transfer_count += 1;
    }

    fn id(&self) -> UserID {
        self.plant_id
    }

    fn transfer_count(&self) -> u128 {
        self.transfer_count
    }
}

pub(crate) struct PowerPlant {
    pub(crate) base: BaseNFT,
}

impl NFT for PowerPlant {
    fn owner(&self) -> UserID {
        self.base.owner()
    }

    fn transfer(&mut self, to: UserID) {
        self.base.transfer(to)
    }

    fn id(&self) -> UserID {
        self.base.id()
    }

    fn transfer_count(&self) -> u128 {
        self.base.transfer_count()
    }
}
