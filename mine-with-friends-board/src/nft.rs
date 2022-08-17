use crate::erc20::ERC20Registry;

use std::ops::IndexMut;

use std::ops::Index;

use crate::erc20::ERC20Ptr;

use super::entity::EntityID;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

pub(crate) trait NFT: Send + Sync {
    fn owner(&self) -> EntityID;
    fn transfer(&mut self, to: EntityID);
    fn id(&self) -> EntityID;
    fn transfer_count(&self) -> u128;
    fn to_json(&self) -> serde_json::Value;
}

pub(crate) type Price = u128;

pub type Currency = ERC20Ptr;

pub(crate) type NFTID = EntityID;

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

#[derive(Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd, Clone, Copy, JsonSchema)]
pub struct NftPtr(EntityID);

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
        tokens: &mut ERC20Registry,
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
    pub(crate) owner: EntityID,
    pub(crate) nft_id: EntityID,
    pub(crate) transfer_count: u128,
}

impl NFT for BaseNFT {
    fn owner(&self) -> EntityID {
        self.owner
    }

    fn transfer(&mut self, to: EntityID) {
        self.owner = to;
        self.transfer_count += 1;
    }

    fn id(&self) -> EntityID {
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
    fn owner(&self) -> EntityID {
        self.base.owner()
    }

    fn transfer(&mut self, to: EntityID) {
        self.base.transfer(to)
    }

    fn id(&self) -> EntityID {
        self.base.id()
    }

    fn transfer_count(&self) -> u128 {
        self.base.transfer_count()
    }

    fn to_json(&self) -> serde_json::Value {
        self.base.to_json()
    }
}
