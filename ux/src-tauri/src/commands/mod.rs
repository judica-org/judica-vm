use crate::{tasks::GameServer, Database, Game, GameState, PrintOnDrop, SigningKeyInner};
use attest_database::db_handle::create::TipControl;
use mine_with_friends_board::{
    entity::EntityID,
    game::game_move::{Chat, Heartbeat, PurchaseNFT, Trade},
    nfts::{sale::UXForSaleList, NftPtr, UXPlantData},
    sanitize::Unsanitized,
};
use sapio_bitcoin::{hashes::hex::ToHex, XOnlyPublicKey};
use schemars::{schema::RootSchema, schema_for};
use std::{path::PathBuf, sync::Arc};
use tauri::{State, Window};
use tokio::sync::futures::Notified;
use tracing::info;
pub mod bindings;
mod modify;
mod view;
