use std::sync::{atomic::AtomicBool, Arc};

use attest_messages::{Authenticated, GenericEnvelope};
use game_player_messages::ParticipantAction;
use sapio_bitcoin::XOnlyPublicKey;
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::task::JoinError;

use crate::{DBFetcher, GenericSequencer, OfflineSequencer, SequenceingError};

#[derive(Deserialize, JsonSchema)]
#[serde(try_from = "OfflineSequencer<ParticipantAction>")]
#[schemars(with = "OfflineSequencer<ParticipantAction>")]
pub struct ExtractedMoveEnvelopes(
    #[schemars(with = "Vec<(ParticipantAction, String)>")]
    pub  Vec<(ParticipantAction, XOnlyPublicKey)>,
);

impl ExtractedMoveEnvelopes {}

impl TryFrom<OfflineSequencer<ParticipantAction>> for ExtractedMoveEnvelopes {
    type Error = SequenceingError<serde_json::Error>;

    fn try_from(mut value: OfflineSequencer<ParticipantAction>) -> Result<Self, Self::Error> {
        let x = value.directly_sequence_map(|m| Ok((m.msg().to_owned(), m.header().key())))?;
        Ok(ExtractedMoveEnvelopes(x))
    }
}

type MoveReadFn = fn(
    Authenticated<GenericEnvelope<ParticipantAction>>,
) -> Result<(ParticipantAction, XOnlyPublicKey), serde_json::Error>;
#[derive(Clone)]
pub struct Sequencer(
    pub  Arc<
        GenericSequencer<
            MoveReadFn,
            (ParticipantAction, XOnlyPublicKey),
            serde_json::Error,
            ParticipantAction,
        >,
    >,
);

fn read_move(
    a: Authenticated<GenericEnvelope<ParticipantAction>>,
) -> Result<(ParticipantAction, XOnlyPublicKey), serde_json::Error> {
    Ok((a.msg().to_owned(), a.header().key()))
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

    pub async fn output_move(&self) -> Option<(ParticipantAction, XOnlyPublicKey)> {
        self.0.output_move().await
    }
}
