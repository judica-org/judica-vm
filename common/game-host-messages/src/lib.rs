use std::collections::VecDeque;

use attest_messages::CanonicalEnvelopeHash;
use mine_with_friends_board::game::{game_move::GameMove, GameSetup};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Peer {
    pub service_url: String,
    pub port: u16,
}
#[derive(Serialize, Deserialize)]
pub enum BroadcastByHost {
    GameSetup(GameSetup),
    Sequence(VecDeque<CanonicalEnvelopeHash>),
    NewPeer(Peer),
    Heartbeat,
}

impl BroadcastByHost {
    pub fn is_sequence(&self) -> bool {
        matches!(self, BroadcastByHost::Sequence(_))
    }
}

#[derive(Serialize, Deserialize)]
pub enum SendToHost {
    AddPeer(Peer),
    MakeMove(GameMove),
}

pub type ChannelID = String;
#[derive(Serialize, Deserialize)]
pub struct Channelized<T> {
    pub data: T,
    pub channel: ChannelID,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
