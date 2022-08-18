use game::game_move::GameMove;
use sanitize::Unsanitized;

mod callbacks;
pub mod entity;
pub mod game;
pub mod nfts;
pub mod sanitize;
pub mod tokens;
pub mod util;

/// Verified is a wrapper for a data type with sequencing and signature data
pub struct MoveEnvelope {
    /// The data
    d: Unsanitized<GameMove>,
    /// The data should be immediately preceded by sequence - 1
    sequence: u64,
    /// a signature which can be verified over d
    sig: String,
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
