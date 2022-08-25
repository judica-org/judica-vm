use sapio_bitcoin::BlockHash;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct BitcoinCheckPoints {
    /// whatever tip hash we've seen recently present if changed where it should
    /// be roughly:
    ///
    /// - Index 0: most recent
    /// - Index 1: 6 ago
    /// - Index 2: 144 ago
    /// - Index 3: 144*7 ago
    /// - Index 4: Arbitrary
    ///
    /// By including these 5, we guarantee a proof of "afterness" withing
    /// reasonable bounds.
    ///
    /// If the hashes are unknown at lower indexes (because of reorg), do not
    /// treat as an error.
    ///
    /// The relative bound between blocks is not checked.
    ///
    /// Even if the hashes haven't changed, we still log them.
    ///
    /// Note that we may already transitively commit to these (or later)
    /// checkpoints via other commitments in the header.
    pub checkpoints: [(BlockHash, i64); 5],
}

impl Default for BitcoinCheckPoints {
    fn default() -> Self {
        Self {
            checkpoints: [(Default::default(), -1); 5],
        }
    }
}
