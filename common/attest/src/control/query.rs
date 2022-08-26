use attest_messages::Envelope;
use sapio_bitcoin::XOnlyPublicKey;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct PushMsg {
    pub msg: Value,
    pub key: XOnlyPublicKey,
}

#[derive(Serialize, Deserialize)]
pub struct Subscribe {
    pub url: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Outcome {
    pub success: bool,
}
