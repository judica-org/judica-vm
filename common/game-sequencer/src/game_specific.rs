use std::sync::{atomic::AtomicBool, Arc};

use attest_messages::{Authenticated, GenericEnvelope};
use game_player_messages::ParticipantAction;
use mine_with_friends_board::MoveEnvelope;
use sapio_bitcoin::XOnlyPublicKey;
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::task::JoinError;

use crate::{DBFetcher, GenericSequencer, OfflineSequencer, SequenceingError};

#[derive(Deserialize, JsonSchema)]
#[serde(try_from = "OfflineSequencer<ParticipantAction>")]
#[schemars(with = "OfflineSequencer<ParticipantAction>")]
pub struct ExtractedMoveEnvelopes(
    #[schemars(with = "Vec<(MoveEnvelope, String)>")] pub Vec<(MoveEnvelope, XOnlyPublicKey)>,
);

impl ExtractedMoveEnvelopes {}

impl TryFrom<OfflineSequencer<ParticipantAction>> for ExtractedMoveEnvelopes {
    type Error = SequenceingError<serde_json::Error>;

    fn try_from(mut value: OfflineSequencer<ParticipantAction>) -> Result<Self, Self::Error> {
        let x = value.directly_sequence_map(read_move)?;
        Ok(ExtractedMoveEnvelopes(x))
    }
}

type MoveReadFn = fn(
    Authenticated<GenericEnvelope<ParticipantAction>>,
) -> Result<Option<(MoveEnvelope, XOnlyPublicKey)>, serde_json::Error>;
#[derive(Clone)]
pub struct Sequencer(
    pub  Arc<
        GenericSequencer<
            MoveReadFn,
            Option<(MoveEnvelope, XOnlyPublicKey)>,
            serde_json::Error,
            ParticipantAction,
        >,
    >,
);

fn read_move(
    m: Authenticated<GenericEnvelope<ParticipantAction>>,
) -> Result<Option<(MoveEnvelope, XOnlyPublicKey)>, serde_json::Error> {
    match m.msg().to_owned() {
        ParticipantAction::MoveEnvelope(me) => Ok(Some((me, m.header().key()))),
        ParticipantAction::Custom(_) => Ok(None),
    }
}

impl Sequencer {
    pub fn new(
        shutdown: Arc<AtomicBool>,
        db_fetcher: Arc<dyn DBFetcher<ParticipantAction>>,
    ) -> Self {
        Sequencer(GenericSequencer::new(shutdown, db_fetcher, read_move))
    }

    pub async fn run(&self) -> Result<(), JoinError> {
        self.0.clone().run().await
    }

    pub async fn output_move(&self) -> Option<(MoveEnvelope, XOnlyPublicKey)> {
        self.0.output_move().await.flatten()
    }
}
