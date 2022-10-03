use std::collections::BTreeMap;

use bitcoin::{hashes::sha256, secp256k1::SecretKey, XOnlyPublicKey};
use sapio::util::amountrange::AmountF64;
use sapio_base::simp::{CompiledObjectLT, SIMPAttachableAt, SIMP};
use schemars::JsonSchema;
use serde::*;
use serde_json::Value;
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct AutoBroadcast {}
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
        0x1
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, Debug)]
#[serde(transparent)]
pub struct PK(#[schemars(with = "sha256::Hash")] pub XOnlyPublicKey);

#[derive(Clone, Serialize, Deserialize, JsonSchema, Debug)]
pub struct GameKernel {
    pub game_host: PK,
    pub players: BTreeMap<PK, AmountF64>,
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
impl DLogSubscription {
    pub fn get_protocol_number() -> i64 {
        0x2
    }
}
impl SIMP for DLogSubscription {
    fn get_protocol_number(&self) -> i64 {
        Self::get_protocol_number()
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
