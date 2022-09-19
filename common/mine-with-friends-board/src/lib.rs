use attest_messages::AttestEnvelopable;
use game::game_move::GameMove;
use ruma_serde::CanonicalJsonValue;
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
/// Verified is a wrapper for a data type with sequencing and signature data
pub struct MoveEnvelope {
    /// The data
    pub d: Unsanitized<GameMove>,
    /// The data should be immediately preceded by sequence - 1
    pub sequence: u64,
    pub time: u64,
}

impl AsRef<MoveEnvelope> for MoveEnvelope {
    fn as_ref(&self) -> &MoveEnvelope {
        self
    }
}
impl AttestEnvelopable for MoveEnvelope {
    type Ref = MoveEnvelope;

    fn as_canonical(&self) -> Result<CanonicalJsonValue, ruma_serde::CanonicalJsonError> {
        ruma_serde::to_canonical_value(self.clone())
    }
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
