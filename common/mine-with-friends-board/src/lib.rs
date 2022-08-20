use game::game_move::GameMove;
use sanitize::Unsanitized;
use serde::{Serialize, Deserialize};

mod callbacks;
pub mod entity;
pub mod game;
pub mod nfts;
pub mod sanitize;
pub mod tokens;
pub mod util;

#[derive(Serialize, Deserialize)]
/// Verified is a wrapper for a data type with sequencing and signature data
pub struct MoveEnvelope {
    /// The data
    d: Unsanitized<GameMove>,
    /// The data should be immediately preceded by sequence - 1
    sequence: u64,
    /// The player who is making the move
    from: entity::EntityID,
    time: u64,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
