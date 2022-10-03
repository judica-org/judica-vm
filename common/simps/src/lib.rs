use std::collections::BTreeMap;

use bitcoin::{hashes::sha256, XOnlyPublicKey};
use sapio::util::amountrange::AmountF64;
use sapio_base::simp::{CompiledObjectLT, SIMPAttachableAt, SIMP};
use schemars::JsonSchema;
use serde::*;
use serde_json::Value;
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct AutoBroadcast {}
impl AutoBroadcast {
    pub fn get_protocol_number() -> i64 {
        -0xcafe
    }
}

impl SIMP for AutoBroadcast {
    fn get_protocol_number(&self) -> i64 {
        Self::get_protocol_number()
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
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct EventSource(pub String);

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct EventKey(pub String);

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
        -0xbeef
    }
}
impl SIMP for EventRecompiler {
    fn get_protocol_number(&self) -> i64 {
        Self::get_protocol_number()
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
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
struct XOnlyPublicKeyString(#[schemars(with = "sha256::Hash")] XOnlyPublicKey);
impl Into<XOnlyPublicKey> for XOnlyPublicKeyString {
    fn into(self) -> XOnlyPublicKey {
        self.0
    }
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AttestContinuationPointSubscription {
    #[schemars(with = "XOnlyPublicKeyString")]
    pub oracle_key: XOnlyPublicKey,
}
impl AttestContinuationPointSubscription {
    pub fn get_protocol_number() -> i64 {
        0x1
    }
}
impl SIMP for AttestContinuationPointSubscription {
    fn get_protocol_number(&self) -> i64 {
        Self::get_protocol_number()
    }
    fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
    fn from_json(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct PK(#[schemars(with = "sha256::Hash")] pub XOnlyPublicKey);

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct GameKernel {
    pub game_host: PK,
    pub players: BTreeMap<PK, AmountF64>,
    pub timeout: u64,
}
impl GameKernel {}
impl SIMP for GameKernel {
    fn get_protocol_number(&self) -> i64 {
        -119
    }
    fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value::<Self>(self.clone())
    }
    fn from_json(v: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(v)
    }
}
impl SIMPAttachableAt<CompiledObjectLT> for GameKernel {}

// Keep in sync with type in mining_game
pub struct GameStarted {
    pub kernel: GameKernel,
}
