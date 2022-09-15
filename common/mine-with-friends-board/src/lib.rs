use game::game_move::GameMove;
use sanitize::Unsanitized;
use serde::{Deserialize, Serialize};

mod callbacks;
pub mod entity;
pub mod game;
pub mod nfts;
pub mod sanitize;
pub mod tokens;
pub mod util;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
/// Verified is a wrapper for a data type with sequencing and signature data
pub struct MoveEnvelope {
    /// The data
    pub d: Unsanitized<GameMove>,
    /// The data should be immediately preceded by sequence - 1
    pub sequence: u64,
    pub time: u64,
}

impl MoveEnvelope {
    pub fn new(d: Unsanitized<GameMove>, sequence: u64, time: u64) -> Self {
        Self { d, sequence, time }
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
