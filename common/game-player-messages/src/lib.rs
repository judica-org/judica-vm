// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use attest_messages::AttestEnvelopable;
use mine_with_friends_board::{game::game_move::GameMove, sanitize::Unsanitized, MoveEnvelope};
use ruma_serde::CanonicalJsonValue;
use sapio_bitcoin::psbt::PartiallySignedTransaction;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub type ChannelID = String;
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Eq, PartialEq)]
pub struct Multiplexed<T> {
    pub data: T,
    pub channel: ChannelID,
}
impl<T> AsRef<Multiplexed<T>> for Multiplexed<T> {
    fn as_ref(&self) -> &Multiplexed<T> {
        self
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Eq, PartialEq)]
#[serde(try_from = "String")]
#[serde(into = "String")]
#[schemars(transparent)]
pub struct PsbtString(#[schemars(with = "String")] pub PartiallySignedTransaction);

impl TryFrom<String> for PsbtString {
    type Error = <PartiallySignedTransaction as FromStr>::Err;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        PartiallySignedTransaction::from_str(&value).map(PsbtString)
    }
}
impl From<PsbtString> for String {
    fn from(val: PsbtString) -> Self {
        val.0.to_string()
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, JsonSchema, Clone)]
/// Verified is a wrapper for a data type with sequencing and signature data
pub enum ParticipantAction {
    MoveEnvelope(MoveEnvelope),
    Custom(#[schemars(with = "serde_json::Value")] CanonicalJsonValue),
    PsbtSigningCoordination(Multiplexed<PsbtString>),
}

impl AsRef<ParticipantAction> for ParticipantAction {
    fn as_ref(&self) -> &ParticipantAction {
        self
    }
}
impl AttestEnvelopable for ParticipantAction {
    type Ref = ParticipantAction;

    fn as_canonical(&self) -> Result<CanonicalJsonValue, ruma_serde::CanonicalJsonError> {
        ruma_serde::to_canonical_value(self.clone())
    }
}

impl From<MoveEnvelope> for ParticipantAction {
    fn from(g: MoveEnvelope) -> Self {
        Self::MoveEnvelope(g)
    }
}
impl ParticipantAction {
    pub fn new(d: Unsanitized<GameMove>, sequence: u64, time_millis: u64) -> Self {
        Self::MoveEnvelope(MoveEnvelope {
            d,
            sequence,
            time_millis,
        })
    }
}
