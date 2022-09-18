use std::sync::{atomic::AtomicBool, Arc};

use attest_messages::{Authenticated, Envelope};
use mine_with_friends_board::{game::game_move::GameMove, MoveEnvelope};
use sapio_bitcoin::XOnlyPublicKey;
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::task::JoinError;

use crate::{DBFetcher, GenericSequencer, OfflineSequencer, SequenceingError};

#[derive(Deserialize, JsonSchema)]
#[serde(try_from = "OfflineSequencer")]
#[schemars(with = "OfflineSequencer")]
pub struct ExtractedMoveEnvelopes(
    #[schemars(with = "Vec<(MoveEnvelope, String)>")] pub Vec<(MoveEnvelope, XOnlyPublicKey)>,
);

impl ExtractedMoveEnvelopes {}

impl TryFrom<OfflineSequencer> for ExtractedMoveEnvelopes {
    type Error = SequenceingError<serde_json::Error>;

    fn try_from(mut value: OfflineSequencer) -> Result<Self, Self::Error> {
        let x = value.directly_sequence_map(|x| {
            Ok((
                serde_json::from_value::<MoveEnvelope>(x.msg().to_owned().into())?,
                x.header().key(),
            ))
        })?;
        Ok(ExtractedMoveEnvelopes(x))
    }
}

type MoveReadFn =
    fn(Authenticated<Envelope>) -> Result<(MoveEnvelope, XOnlyPublicKey), serde_json::Error>;
#[derive(Clone)]
pub struct Sequencer(
    Arc<GenericSequencer<MoveReadFn, (MoveEnvelope, XOnlyPublicKey), serde_json::Error>>,
);

fn read_move(
    a: Authenticated<Envelope>,
) -> Result<(MoveEnvelope, XOnlyPublicKey), serde_json::Error> {
    Ok((
        serde_json::from_value(a.msg().to_owned().into())?,
        a.header().key(),
    ))
}

impl Sequencer {
    pub fn new(shutdown: Arc<AtomicBool>, db_fetcher: Arc<dyn DBFetcher>) -> Self {
        Sequencer(GenericSequencer::new(shutdown, db_fetcher, read_move))
    }

    pub async fn run(&self) -> Result<(), JoinError> {
        self.0.clone().run().await
    }

    pub async fn output_move(&self) -> Option<(MoveEnvelope, XOnlyPublicKey)> {
        self.0.output_move().await
    }
}
