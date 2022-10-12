// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{Database, GameState, SigningKeyInner};

use mine_with_friends_board::{
    entity::EntityID,
    game::game_move::{Chat, PurchaseNFT, Trade},
};
use sapio_bitcoin::XOnlyPublicKey;
use schemars::{schema::RootSchema, schema_for};
use std::path::PathBuf;
use tauri::{State, Window};

pub mod bindings;
pub mod modify;
pub mod view;
