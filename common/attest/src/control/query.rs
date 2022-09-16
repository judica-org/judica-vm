use ruma_serde::CanonicalJsonValue;
use sapio_bitcoin::XOnlyPublicKey;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PushMsg {
    pub msg: CanonicalJsonValue,
    pub key: XOnlyPublicKey,
}

#[derive(Serialize, Deserialize)]
pub struct Subscribe {
    pub url: String,
    pub port: u16,
    #[serde(default)]
    pub fetch_from: Option<bool>,
    #[serde(default)]
    pub push_to: Option<bool>,
    #[serde(default)]
    pub allow_unsolicited_tips: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Outcome {
    pub success: bool,
}
