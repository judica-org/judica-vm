use serde::{Deserialize, Serialize};
pub mod chain_commit_groups;
pub mod hidden_services;
pub mod messages;
pub mod nonces;
pub mod users;
#[derive(Serialize, Deserialize)]
pub struct PeerInfo {
    pub service_url: String,
    pub port: u16,
    pub fetch_from: bool,
    pub push_to: bool,
    pub allow_unsolicited_tips: bool,
}
