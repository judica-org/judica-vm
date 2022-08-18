mod callbacks;
pub mod entity;
pub mod game;
pub mod nfts;
pub mod sanitize;
pub mod tokens;
pub mod util;

/// Verified is a wrapper for a data type with sequencing and signature data
pub struct Verified<D> {
    /// The data
    d: D,
    /// The data should be immediately preceded by sequence - 1
    sequence: u64,
    /// a signature which can be verified over d
    sig: String,
    /// The player who is making the move
    from: entity::EntityID,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
