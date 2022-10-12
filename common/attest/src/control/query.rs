// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ruma_serde::CanonicalJsonValue;
use sapio_bitcoin::XOnlyPublicKey;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PushMsg {
    pub msg: CanonicalJsonValue,
    pub key: XOnlyPublicKey,
    #[serde(default)]
    pub equivocate: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Subscribe {
    pub url: String,
    pub port: u16,
    #[serde(default)]
    pub fetch_from: Option<bool>,
    #[serde(default)]
    pub push_to: Option<bool>,
    #[serde(default)]
    pub allow_unsolicited_tips: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Outcome {
    pub success: bool,
}

#[derive(Serialize, Deserialize)]
pub struct NewGenesis {
    pub nickname: String,
    pub msg: CanonicalJsonValue,
    #[serde(default)]
    pub danger_extended_private_key: Option<String>,
}
