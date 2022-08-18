use self::instances::powerplant::PowerPlant;

use super::entity::EntityID;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::cmp::min;
use std::collections::BTreeMap;
use std::ops::Index;
use std::ops::IndexMut;
pub mod instances;
pub mod sale;
pub(crate) trait NFT: Send + Sync {
    fn owner(&self) -> EntityID;
    fn transfer(&mut self, to: EntityID);
    fn id(&self) -> EntityID;
    fn transfer_count(&self) -> u128;
    fn to_json(&self) -> serde_json::Value;
}

pub(crate) type NFTID = EntityID;

#[derive(Default)]
pub(crate) struct NFTRegistry {
    pub nfts: BTreeMap<NftPtr, Box<dyn NFT>>,
    pub power_plants: BTreeMap<NftPtr, PowerPlant>,
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

#[derive(Serialize, Clone)]
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
        if self.transfer_count() == u128::max_value() {
            return;
        }
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

macro_rules! NFT_BASE {
    ($i:ident) => {
        impl NFT for $i {
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
    };
}
