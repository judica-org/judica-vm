// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use bitcoin::hashes::hex::ToHex;
use bitcoin::hashes::sha256;
use bitcoin::hashes::Hash;
use bitcoin::Amount;
use bitcoin::OutPoint;
use bitcoin::Transaction;
use bitcoin::XOnlyPublicKey;
use event_log::db_handle::accessors::occurrence::ApplicationTypeID;
use event_log::db_handle::accessors::occurrence::Occurrence;
use event_log::db_handle::accessors::occurrence::OccurrenceConversionError;
use event_log::db_handle::accessors::occurrence::ToOccurrence;
use event_log::db_handle::accessors::occurrence_group::OccurrenceGroupKey;
use game_player_messages::PsbtString;
use ruma_serde::CanonicalJsonValue;
use sapio::util::amountrange::AmountU64;
use sapio_base::plugin_args::ContextualArguments;
use sapio_base::plugin_args::CreateArgs;
use sapio_base::serialization_helpers::SArc;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use simps::EventKey;
use simps::GameKernel;
use simps::GameStarted;
use simps::PK;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    ModuleBytes(OccurrenceGroupKey, String),
    CreateArgs(CreateArgs<Value>),
    TransactionFinalized(String, Transaction),
    Rebind(OutPoint),
    SyntheticPeriodicActions(i64),
    NewRecompileTriggeringObservation(Value, SArc<EventKey>),
    // strictly speaking we don't need this to be an event with any information.
    EmittedPSBTVia(PsbtString, XOnlyPublicKey),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Tag {
    InitModule,
    CreateArgs,
    FirstBind,
    EvLoopCounter(u64),
    ScopedCounter(String, u64),
    ScopedValue(String, String),
}

impl ToString for Tag {
    fn to_string(&self) -> String {
        ruma_serde::to_canonical_value(self)
            .expect("Tag Type Should not have Fallible Serialization")
            .to_string()
    }
}

#[derive(Serialize, Deserialize)]
pub struct TaggedEvent(pub Event, pub Option<Tag>);

impl ToOccurrence for TaggedEvent {
    fn to_data(&self) -> CanonicalJsonValue {
        ruma_serde::to_canonical_value(&self.0).unwrap()
    }
    fn stable_typeid() -> ApplicationTypeID {
        ApplicationTypeID::from_inner("LitigatorEvent")
    }
    fn unique_tag(&self) -> Option<String> {
        self.1.as_ref().map(ToString::to_string)
    }
    fn from_occurrence(occurrence: Occurrence) -> Result<TaggedEvent, OccurrenceConversionError>
    where
        Self: Sized + for<'de> Deserialize<'de>,
    {
        let v: Event = serde_json::from_value(occurrence.data.into())
            .map_err(OccurrenceConversionError::DeserializationError)?;
        if occurrence.typeid != Self::stable_typeid() {
            return Err(OccurrenceConversionError::TypeidMismatch {
                expected: occurrence.typeid,
                got: Self::stable_typeid(),
            });
        }
        let tag = occurrence
            .unique_tag
            .map(|t| serde_json::from_str(&t))
            .transpose()
            .map_err(OccurrenceConversionError::DeserializationError)?;
        Ok(TaggedEvent(v, tag))
    }
}

pub fn convert_setup_to_contract_args(
    setup: mine_with_friends_board::game::GameSetup,
    oracle_key: &XOnlyPublicKey,
) -> Result<CreateArgs<Value>, bitcoin::secp256k1::Error> {
    let amt_per_player: AmountU64 =
        AmountU64::from(Amount::from_sat(100000 / setup.players.len() as u64));
    let g = GameKernel {
        game_host: PK(*oracle_key),
        players: setup
            .players
            .iter()
            .map(|p| Ok((PK(XOnlyPublicKey::from_str(p)?), amt_per_player)))
            .collect::<Result<_, bitcoin::secp256k1::Error>>()?,
        timeout: setup.finish_time,
    };
    let args = CreateArgs {
        arguments: serde_json::to_value(&GameStarted { kernel: g }).unwrap(),
        context: ContextualArguments {
            network: bitcoin::network::constants::Network::Bitcoin,
            amount: Amount::from_sat(100000),
            effects: Default::default(),
        },
    };
    Ok(args)
}

#[derive(Serialize, Deserialize)]
pub struct ModuleRepo(pub Vec<u8>);

impl ModuleRepo {
    pub fn default_group_key() -> OccurrenceGroupKey {
        ModuleRepo::stable_typeid().into_inner()
    }
}

impl ToOccurrence for ModuleRepo {
    fn to_data(&self) -> CanonicalJsonValue {
        ruma_serde::to_canonical_value(self).unwrap()
    }

    fn stable_typeid() -> ApplicationTypeID
    where
        Self: Sized,
    {
        ApplicationTypeID::from_inner("ModuleRepo")
    }

    fn unique_tag(&self) -> Option<String> {
        Some(sha256::Hash::hash(&self.0[..]).to_hex())
    }
}
