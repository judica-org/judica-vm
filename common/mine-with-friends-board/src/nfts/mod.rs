use self::instances::powerplant::{PlantType, PowerPlant};
use crate::util::{ForSale, HasMiners, Location, Watts};

use super::entity::EntityID;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use std::collections::BTreeMap;
use std::ops::Index;
use std::ops::IndexMut;
pub mod instances;
pub mod sale;
/// All NFTs must implement these behaviors
pub(crate) trait NFT: Send + Sync {
    /// Return the EntityID of the current Owner
    fn owner(&self) -> EntityID;
    /// Transfer the NFT from the current Owner to someone else
    fn transfer(&mut self, to: EntityID);
    /// Get the EntityID of the
    fn id(&self) -> EntityID;
    /// How many times has this NFT been transfered
    fn transfer_count(&self) -> u128;
    /// Represent the NFT as a JSON
    fn to_json(&self) -> serde_json::Value;
}

/// A Registry of all NFTs and their MetaData
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

/// A special Pointer designed for safer access to the NFTRegistry (prevent
/// confusion with EntityID type)
///
/// TODO: Guarantee validity for a given NFTRegistry
#[derive(
    Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd, Clone, Copy, JsonSchema, Debug,
)]
pub struct NftPtr(EntityID);

impl NFTRegistry {
    pub(crate) fn add(&mut self, nft: Box<dyn NFT>) -> NftPtr {
        let id = NftPtr(nft.id());
        if let std::collections::btree_map::Entry::Vacant(e) = self.nfts.entry(id) {
            e.insert(nft);
        } else {
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

/// Basic NFT Implementation
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

#[derive(Serialize, Clone, Debug)]
pub struct UXPlantData {
    pub coordinates: Location,
    pub for_sale: ForSale,
    pub has_miners: HasMiners,
    pub owner: EntityID,
    pub plant_type: PlantType,
    pub watts: Watts,
    pub hashrate: u128,
}
#[derive(Serialize, Clone)]
pub struct UXNFTRegistry {
    pub power_plant_data: BTreeMap<NftPtr, UXPlantData>,
}
