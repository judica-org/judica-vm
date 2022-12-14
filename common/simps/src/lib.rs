// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use bitcoin::{
    hashes::{hex::ToHex, sha256},
    secp256k1::SecretKey,
    BlockHash, BlockHeader, TxMerkleNode, XOnlyPublicKey,
};
use lazy_static::lazy_static;
use sapio::util::amountrange::AmountU64;
use sapio_base::{
    serialization_helpers::SArc,
    simp::{CompiledObjectLT, ContinuationPointLT, SIMPAttachableAt, TemplateLT, SIMP},
};
use schemars::JsonSchema;
use serde::*;
use serde_json::Value;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct AutBroadcastOptions {
    pub sign: bool,
    pub sign_all: bool,
    pub send: bool,
    pub finalize: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct AutoBroadcast {
    pub signer_roles: Vec<(PK, AutBroadcastOptions)>,
}
impl AutoBroadcast {
    pub fn get_protocol_number() -> i64 {
        Self::static_get_protocol_number()
    }
}
impl SIMP for AutoBroadcast {
    fn get_protocol_number(&self) -> i64 {
        Self::static_get_protocol_number()
    }

    fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    fn from_json(value: serde_json::Value) -> Result<Self, serde_json::Error>
    where
        Self: Sized,
    {
        serde_json::from_value(value)
    }

    fn static_get_protocol_number() -> i64
    where
        Self: Sized,
    {
        -0xcafe
    }
}

impl SIMPAttachableAt<TemplateLT> for AutoBroadcast {}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct EventSource(pub String);

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct EventKey(pub String);
impl EventKey {
    pub fn is_wildcard(&self) -> bool {
        self.0 == "*"
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct EventRecompiler {
    pub source: EventSource,
    pub filter: EventKey,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Event {
    pub key: EventKey,
    pub slot: String,
    pub data: Value,
}

impl EventRecompiler {
    pub fn get_protocol_number() -> i64 {
        Self::static_get_protocol_number()
    }
}
impl SIMP for EventRecompiler {
    fn get_protocol_number(&self) -> i64 {
        Self::static_get_protocol_number()
    }
    fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    fn from_json(value: serde_json::Value) -> Result<Self, serde_json::Error>
    where
        Self: Sized,
    {
        serde_json::from_value(value)
    }

    fn static_get_protocol_number() -> i64
    where
        Self: Sized,
    {
        -0xbeef
    }
}

impl SIMPAttachableAt<ContinuationPointLT> for EventRecompiler {}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
struct XOnlyPublicKeyString(#[schemars(with = "sha256::Hash")] XOnlyPublicKey);
impl From<XOnlyPublicKeyString> for XOnlyPublicKey {
    fn from(val: XOnlyPublicKeyString) -> Self {
        val.0
    }
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AttestContinuationPointSubscription {
    #[schemars(with = "XOnlyPublicKeyString")]
    pub oracle_key: XOnlyPublicKey,
}
impl AttestContinuationPointSubscription {
    pub fn get_protocol_number() -> i64 {
        Self::static_get_protocol_number()
    }
}
impl SIMP for AttestContinuationPointSubscription {
    fn get_protocol_number(&self) -> i64 {
        Self::static_get_protocol_number()
    }
    fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
    fn from_json(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }

    fn static_get_protocol_number() -> i64
    where
        Self: Sized,
    {
        -0xa7735c09055
    }
}

#[derive(
    Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, Debug, Copy,
)]
#[serde(transparent)]
pub struct PK(#[schemars(with = "sha256::Hash")] pub XOnlyPublicKey);

#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct GameKernel {
    pub game_host: PK,
    pub players: BTreeMap<PK, AmountU64>,
    pub timeout: u64,
}
impl GameKernel {}
impl SIMP for GameKernel {
    fn get_protocol_number(&self) -> i64 {
        Self::static_get_protocol_number()
    }
    fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value::<Self>(self.clone())
    }
    fn from_json(v: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(v)
    }

    fn static_get_protocol_number() -> i64
    where
        Self: Sized,
    {
        -119
    }
}
impl SIMPAttachableAt<CompiledObjectLT> for GameKernel {}

// Keep in sync with type in mining_game
#[derive(Serialize)]
pub struct GameStarted {
    pub kernel: GameKernel,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub struct DLogDiscovered {
    pub dlog_discovered: SecretKey,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub struct DLogSubscription {
    pub dlog_subscription: PK,
}
impl DLogSubscription {}
impl SIMP for DLogSubscription {
    fn static_get_protocol_number() -> i64 {
        -0xd15c12337109
    }

    fn get_protocol_number(&self) -> i64 {
        Self::static_get_protocol_number()
    }

    fn to_json(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    fn from_json(value: Value) -> Result<Self, serde_json::Error>
    where
        Self: Sized,
    {
        serde_json::from_value(value)
    }
}
impl SIMPAttachableAt<ContinuationPointLT> for DLogSubscription {}

lazy_static! {
    pub static ref EK_WILDCARD: SArc<EventKey> =
        SArc(Arc::new(EventKey("*".into())));
    pub static ref EK_GAME_ACTION_WIN: SArc<EventKey> =
        SArc(Arc::new(EventKey("game_action_players_win".into())));
    pub static ref EK_GAME_ACTION_LOSE: SArc<EventKey> =
        SArc(Arc::new(EventKey("game_action_players_lose".into())));
    // Bitcoin Related
    pub static ref SOURCE_BITCOIN_RPC: SArc<EventSource> =
        SArc(Arc::new(EventSource("bitcoin-rpc".into())));
    pub static ref EK_BITCOIN_BLOCKHEADER: SArc<EventKey> =
        SArc(Arc::new(EventKey("blockheader".into())));
}
pub fn ek_new_dlog(x: XOnlyPublicKey) -> EventKey {
    EventKey(format!("discrete_log_discovery({})", x.to_hex()))
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ListenBlockHeader {
    #[schemars(with = "BlockHeaderRemote")]
    pub current_block_header: BlockHeader,
    #[schemars(with = "BlockHeaderRemote")]
    pub parent_in: BlockHeader,
}

#[derive(JsonSchema)]
#[serde(remote = "BlockHeader")]
pub struct BlockHeaderRemote {
    /// The protocol version. Should always be 1.
    pub version: i32,
    /// Reference to the previous block in the chain.
    #[schemars(with = "bitcoin::hashes::sha256d::Hash")]
    pub prev_blockhash: BlockHash,
    /// The root hash of the merkle tree of transactions in the block.
    #[schemars(with = "bitcoin::hashes::sha256d::Hash")]
    pub merkle_root: TxMerkleNode,
    /// The timestamp of the block, as claimed by the miner.
    pub time: u32,
    /// The target value below which the blockhash must lie, encoded as a
    /// a float (with well-defined rounding, of course).
    pub bits: u32,
    /// The nonce, selected to obtain a low enough blockhash.
    pub nonce: u32,
}
