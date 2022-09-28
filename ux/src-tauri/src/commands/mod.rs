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
