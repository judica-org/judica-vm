// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use attest_messages::{AttestEnvelopable, CanonicalEnvelopeHash};
use mine_with_friends_board::game::{game_move::GameMove, GameSetup};
use ruma_serde::CanonicalJsonValue;
use sapio_bitcoin::{
    hashes::hex::{FromHex, ToHex},
    secp256k1::rand::{thread_rng, Rng},
    XOnlyPublicKey,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::VecDeque,
    error::Error,
    fmt::{Debug, Display},
};

#[derive(Deserialize, Serialize)]
pub struct FinishArgs {
    pub passcode: JoinCode,
    pub code: JoinCode,
    pub start_amount: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewGameArgs {
    pub duration_minutes: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewGame {
    pub password: JoinCode,
    pub join: JoinCode,
}
#[derive(Debug, Serialize, Deserialize)]
pub enum AddPlayerError {
    AlreadySetup,
    NoMorePlayers,
    NotGenesisEnvelope,
    WrongFirstMessage,
}
impl Display for AddPlayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}
impl Error for AddPlayerError {}

#[derive(Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd, Hash, Clone, Copy, JsonSchema)]
#[serde(into = "String")]
#[serde(try_from = "String")]
pub struct JoinCode(#[schemars(with = "String")] [u8; 16]);

impl Debug for JoinCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("JoinCode")
            .field(&String::from(*self))
            .finish()
    }
}

impl TryFrom<String> for JoinCode {
    type Error = sapio_bitcoin::hashes::hex::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        FromHex::from_hex(&value).map(JoinCode)
    }
}
impl From<JoinCode> for String {
    fn from(s: JoinCode) -> Self {
        s.0.to_hex()
    }
}
impl JoinCode {
    fn new() -> Self {
        let mut rng = thread_rng();
        JoinCode(rng.gen())
    }
}

impl Default for JoinCode {
    fn default() -> Self {
        Self::new()
    }
}
#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone)]
pub struct Peer {
    pub service_url: String,
    pub port: u16,
}
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub enum BroadcastByHost {
    GameSetup(GameSetup),
    Sequence(VecDeque<CanonicalEnvelopeHash>),
    NewPeer(Peer),
    Heartbeat,
}

impl BroadcastByHost {
    pub fn is_sequence(&self) -> bool {
        matches!(self, BroadcastByHost::Sequence(_))
    }
}

#[derive(Serialize, Deserialize)]
pub enum SendToHost {
    AddPeer(Peer),
    MakeMove(GameMove),
}

pub type ChannelID = String;
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Channelized<T> {
    pub data: T,
    pub channel: ChannelID,
}
impl<T> AsRef<Channelized<T>> for Channelized<T> {
    fn as_ref(&self) -> &Channelized<T> {
        self
    }
}

impl<T> AttestEnvelopable for Channelized<T>
where
    T: Send + Sync + std::fmt::Debug + Clone + JsonSchema + Serialize + for<'de> Deserialize<'de>,
{
    type Ref = Self;

    fn as_canonical(&self) -> Result<CanonicalJsonValue, ruma_serde::CanonicalJsonError> {
        ruma_serde::to_canonical_value(self)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

#[derive(Serialize, Deserialize)]
pub struct CreatedNewChain {
    pub sequencer_key: XOnlyPublicKey,
    pub genesis_hash: CanonicalEnvelopeHash,
    pub group_name: String,
}

#[derive(Serialize, Deserialize)]
pub struct FetchedLit(pub Vec<Value>, pub Value);
