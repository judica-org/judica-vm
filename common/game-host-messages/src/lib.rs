use attest_messages::{AttestEnvelopable, CanonicalEnvelopeHash};
use mine_with_friends_board::game::{game_move::GameMove, GameSetup};
use ruma_serde::CanonicalJsonValue;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone)]
pub struct Peer {
    pub service_url: String,
    pub port: u16,
}
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
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
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Channelized<T> {
    pub data: T,
    pub channel: ChannelID,
}
impl<T> AsRef<Channelized<T>> for Channelized<T> {
    fn as_ref(&self) -> &Channelized<T> {
        self
    }
}

impl<T: Send + Sync + std::fmt::Debug + Clone + JsonSchema> AttestEnvelopable for Channelized<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    type Ref = Self;

    fn as_canonical(&self) -> Result<CanonicalJsonValue, ruma_serde::CanonicalJsonError> {
        ruma_serde::to_canonical_value(self)
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
