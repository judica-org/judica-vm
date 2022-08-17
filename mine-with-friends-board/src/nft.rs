use crate::erc20::ERC20Registry;

use std::ops::IndexMut;

use std::ops::Index;

use crate::erc20::ERC20Ptr;

use super::UserID;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

pub(crate) trait NFT : Send + Sync {
    fn owner(&self) -> UserID;
    fn transfer(&mut self, to: UserID);
    fn id(&self) -> UserID;
    fn transfer_count(&self) -> u128;
    fn to_json(&self) -> serde_json::Value;
}

pub(crate) type Price = u128;

pub type Currency = ERC20Ptr;

pub(crate) type NFTID = UserID;


#[derive(Default)]
pub(crate) struct NFTRegistry {
    pub(crate) nfts: BTreeMap<NftPtr, Box<dyn NFT>>,
}
impl Serialize for NFTRegistry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.nfts.values().map(|n| n.to_json()))
    }
}

#[derive(Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd, Clone, Copy)]
pub struct NftPtr(UserID);

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

#[derive(Serialize, Default)]
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

#[derive(Serialize)]
pub(crate) struct BaseNFT {
    pub(crate) owner: UserID,
    pub(crate) nft_id: UserID,
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
        self.nft_id
    }

    fn transfer_count(&self) -> u128 {
        self.transfer_count
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
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

    fn to_json(&self) -> serde_json::Value {
        self.base.to_json()
    }
}
