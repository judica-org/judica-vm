use attest_messages::CanonicalEnvelopeHash;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Tips {
    pub tips: Vec<CanonicalEnvelopeHash>,
}
