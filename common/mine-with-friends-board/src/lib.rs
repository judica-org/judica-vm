use game::game_move::GameMove;

use sanitize::Unsanitized;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod callbacks;
pub mod entity;
pub mod game;
pub mod nfts;
pub mod sanitize;
pub mod tokens;
pub mod util;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, JsonSchema, Clone)]
pub struct MoveEnvelope {
    /// The data
    pub d: Unsanitized<GameMove>,
    /// The data should be immediately preceded by sequence - 1
    pub sequence: u64,
    #[serde(alias = "time")]
    pub time_millis: u64,
}

#[cfg(test)]
mod tests;
