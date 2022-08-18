use crate::callbacks::Callback;
use crate::callbacks::CallbackRegistry;
use crate::entity::EntityIDAllocator;
use crate::erc20::ERC20Registry;
use crate::game::GameBoard;

use std::any::Any;
use std::cmp::min;
use std::fmt::format;
use std::hash::Hash;
use std::ops::IndexMut;

use std::ops::Index;
use std::rc::Rc;
use std::sync::Arc;

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

#[derive(Clone)]
pub struct PowerPlantEvent {
    pub time: u64,
    pub period: u64,
}
impl Callback for PowerPlantEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn action(&mut self, game: &mut GameBoard) {
        let plants = game.nfts.power_plants.clone();
        let mut total = 0;
        let mut shares: BTreeMap<EntityID, u128> = BTreeMap::new();
        for (id, plant) in plants {
            let share = plant.compute_hashrate(game);
            total += share;
            let owner = game.nfts[id].owner();
            *shares.entry(owner).or_default() += share;
        }
        shares
            .values_mut()
            .for_each(|v| *v = ((*v * 1024 * game.mining_subsidy) / total) / 1024);

        let btc = &mut game.erc20s[game.bitcoin_token_id.unwrap()];
        btc.transaction();
        for (to, amount) in shares {
            btc.mint(&to, amount)
        }
        btc.end_transaction();

        // Reschedule
        self.time += self.period;
        game.callbacks.schedule(Box::new(self.clone()));
    }

    fn purpose(&self) -> String {
        todo!()
    }
}

#[derive(Serialize, Clone)]
pub enum PlantType {
    Coal,
    Solar,
    Hydro,
    Nuclear,
    Geothermal,
    Flare,
}
#[derive(Serialize, Clone)]
pub(crate) struct PowerPlant {
    pub id: NftPtr,
    pub plant_type: PlantType,
    pub watts: u128,
    pub coordinates: (u64, u64),
}

struct Hashboard();
impl PowerPlant {
    fn compute_hashrate(&self, game: &mut GameBoard) -> u128 {
        // TODO: Some algo that uses watts / coordinates / plant_type to compute a scalar?
        let scale = 1000;
        let mut hash = Vec::with_capacity(game.erc20s.hashboards.len());
        let hashers: Vec<_> = game.erc20s.hashboards.keys().cloned().collect();
        for token in hashers {
            if let Some(hbd) = game.erc20s.hashboards.get(&token) {
                let hpw = hbd.hash_per_watt;
                let count = game.erc20s[token].balance_check(&self.id.0);
                hash.push((hpw, count));
            }
        }
        hash.sort_unstable();
        let mut watts = self.watts;
        let mut hashrate = 0;
        while let Some((hpw, units)) = hash.pop() {
            let available = min(units, watts);
            hashrate += available * hpw;
            watts -= available;
            if watts == 0 {
                break;
            }
        }
        hashrate
    }
    fn colocate_hashrate(&self, game: &mut GameBoard, miners: ERC20Ptr, amount: Price) {
        let owner = game.nfts[self.id].owner();
        game.erc20s[miners].transaction();
        let _ = game.erc20s[miners].transfer(&owner, &self.id.0, amount);
        game.erc20s[miners].end_transaction();
    }
    /// Withdrawals are processed via a CoinLockup which emulates shipping
    fn ship_hashrate(
        &self,
        tokens: &mut ERC20Registry,
        miners: ERC20Ptr,
        amount: Price,
        nfts: &mut NFTRegistry,
        alloc: &mut EntityIDAllocator,
        shipping_time: u64,
        game: &mut GameBoard,
    ) {
        let owner = game.nfts[self.id].owner();
        let lockup = CoinLockup {
            base: BaseNFT {
                owner: owner,
                nft_id: alloc.make(),
                // Non Transferrable
                transfer_count: u128::max_value(),
            },
            time_when_free: shipping_time,
            asset: miners,
        };
        let p = nfts.add(Box::new(lockup.clone()));
        game.callbacks.schedule(Box::new(lockup.clone()));
        tokens[miners].transaction();
        let _ = tokens[miners].transfer(&self.id.0, &p.0, amount);
        tokens[miners].end_transaction();
    }
}

#[derive(Serialize, Clone)]
pub(crate) struct CoinLockup {
    pub(crate) base: BaseNFT,
    time_when_free: u64,
    asset: ERC20Ptr,
}
NFT_BASE!(CoinLockup);
impl CoinLockup {
    // Note: just reads immutable fields, modifies external state
    fn unlock(&self, tokens: &mut ERC20Registry, current_time: u64) {
        if current_time < self.time_when_free {
            return;
        }
        let token = &mut tokens[self.asset];
        token.transaction();
        let balance = token.balance_check(&self.id());
        let _ = token.transfer(&self.id(), &self.owner(), balance);
        token.end_transaction();
    }
}

impl Callback for CoinLockup {
    fn time(&self) -> u64 {
        self.time_when_free
    }

    fn action(&mut self, game: &mut GameBoard) {
        self.unlock(&mut game.erc20s, game.current_time)
    }

    fn purpose(&self) -> String {
        format!("CoinLockup Release Trigger")
    }
}
