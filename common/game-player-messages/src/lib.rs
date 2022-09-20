use attest_messages::AttestEnvelopable;
use mine_with_friends_board::{game::game_move::GameMove, sanitize::Unsanitized, MoveEnvelope};
use ruma_serde::CanonicalJsonValue;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, JsonSchema, Clone)]
/// Verified is a wrapper for a data type with sequencing and signature data
pub enum ParticipantAction {
    MoveEnvelope(MoveEnvelope),
    Custom(#[schemars(with = "serde_json::Value")] CanonicalJsonValue),
}

impl AsRef<ParticipantAction> for ParticipantAction {
    fn as_ref(&self) -> &ParticipantAction {
        self
    }
}
impl AttestEnvelopable for ParticipantAction {
    type Ref = ParticipantAction;

    fn as_canonical(&self) -> Result<CanonicalJsonValue, ruma_serde::CanonicalJsonError> {
        ruma_serde::to_canonical_value(self.clone())
    }
}

impl From<MoveEnvelope> for ParticipantAction {
    fn from(g: MoveEnvelope) -> Self {
        Self::MoveEnvelope(g)
    }
}
impl ParticipantAction {
    pub fn new(d: Unsanitized<GameMove>, sequence: u64, time: u64) -> Self {
        Self::MoveEnvelope(MoveEnvelope { d, sequence, time })
    }
}
