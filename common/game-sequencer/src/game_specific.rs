use std::sync::{atomic::AtomicBool, Arc};

use attest_messages::{Authenticated, Envelope, GenericEnvelope};
use mine_with_friends_board::{game::game_move::GameMove, MoveEnvelope};
use sapio_bitcoin::XOnlyPublicKey;
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::task::JoinError;

use crate::{DBFetcher, GenericSequencer, OfflineSequencer, SequenceingError};

#[derive(Deserialize, JsonSchema)]
#[serde(try_from = "OfflineSequencer<MoveEnvelope>")]
#[schemars(with = "OfflineSequencer<MoveEnvelope>")]
pub struct ExtractedMoveEnvelopes(
    #[schemars(with = "Vec<(MoveEnvelope, String)>")] pub Vec<(MoveEnvelope, XOnlyPublicKey)>,
);

impl ExtractedMoveEnvelopes {}

impl TryFrom<OfflineSequencer<MoveEnvelope>> for ExtractedMoveEnvelopes {
    type Error = SequenceingError<serde_json::Error>;

    fn try_from(mut value: OfflineSequencer<MoveEnvelope>) -> Result<Self, Self::Error> {
        let x = value.directly_sequence_map(|m| Ok((m.msg().to_owned(), m.header().key())))?;
        Ok(ExtractedMoveEnvelopes(x))
    }
}

type MoveReadFn = fn(
    Authenticated<GenericEnvelope<MoveEnvelope>>,
) -> Result<(MoveEnvelope, XOnlyPublicKey), serde_json::Error>;
#[derive(Clone)]
pub struct Sequencer(
    Arc<
        GenericSequencer<
            MoveReadFn,
            (MoveEnvelope, XOnlyPublicKey),
            serde_json::Error,
            MoveEnvelope,
        >,
    >,
);

fn read_move(
    a: Authenticated<GenericEnvelope<MoveEnvelope>>,
) -> Result<(MoveEnvelope, XOnlyPublicKey), serde_json::Error> {
    Ok((a.msg().to_owned(), a.header().key()))
}

impl Sequencer {
    pub fn new(shutdown: Arc<AtomicBool>, db_fetcher: Arc<dyn DBFetcher<MoveEnvelope>>) -> Self {
        Sequencer(GenericSequencer::new(shutdown, db_fetcher, read_move))
    }

    pub async fn run(&self) -> Result<(), JoinError> {
        self.0.clone().run().await
    }

    pub async fn output_move(&self) -> Option<(MoveEnvelope, XOnlyPublicKey)> {
        self.0.output_move().await
    }
}
