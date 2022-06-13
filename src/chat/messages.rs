use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InnerMessage {
    Ping(String),
    Data(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Envelope {
    pub key: ed25519_dalek::PublicKey,
    pub channel: String,
    pub sent_time_ms: u64,
    #[serde(default)]
    pub signatures: ruma_signatures::PublicKeyMap,
    pub msg: InnerMessage,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageResponse {
    Pong(String),
    None,
}
