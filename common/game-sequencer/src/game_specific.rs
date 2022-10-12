// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use attest_messages::{Authenticated, GenericEnvelope};
use game_player_messages::ParticipantAction;
use mine_with_friends_board::MoveEnvelope;
use ruma_serde::CanonicalJsonValue;
use sapio_bitcoin::{psbt::PartiallySignedTransaction, XOnlyPublicKey};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::{atomic::AtomicBool, Arc};
#[cfg(feature = "has_async")]
use tokio::{
    spawn,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
    task::JoinError,
};

#[cfg(feature = "has_async")]
use crate::DBFetcher;
#[cfg(feature = "has_async")]
use crate::GenericSequencer;
use crate::OfflineSequencer;
use crate::SequenceingError;

#[derive(Deserialize)]
#[serde(try_from = "OfflineSequencer<ParticipantAction>")]
pub struct ExtractedMoveEnvelopes(pub Vec<(MoveEnvelope, XOnlyPublicKey)>);

impl JsonSchema for ExtractedMoveEnvelopes {
    fn schema_name() -> String {
        OfflineSequencer::<ParticipantAction>::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        OfflineSequencer::<ParticipantAction>::json_schema(gen)
    }
}

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
#[cfg(feature = "has_async")]
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
        ParticipantAction::PsbtSigningCoordination(_) => Ok(None),
    }
}

#[cfg(feature = "has_async")]
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

type AGP = Authenticated<GenericEnvelope<ParticipantAction>>;
type EnvReadFn = fn(AGP) -> Result<AGP, serde_json::Error>;
#[cfg(feature = "has_async")]
#[derive(Clone)]
pub struct DemuxedSequencer {
    pub sequencer: Arc<GenericSequencer<EnvReadFn, AGP, serde_json::Error, ParticipantAction>>,
    pub send_move: UnboundedSender<(MoveEnvelope, XOnlyPublicKey)>,
    pub recieve_move: Arc<Mutex<UnboundedReceiver<(MoveEnvelope, XOnlyPublicKey)>>>,
    pub send_psbt: UnboundedSender<(PartiallySignedTransaction, String)>,
    pub recieve_psbt: Arc<Mutex<UnboundedReceiver<(PartiallySignedTransaction, String)>>>,
    pub send_custom: UnboundedSender<CanonicalJsonValue>,
    pub recieve_custom: Arc<Mutex<UnboundedReceiver<CanonicalJsonValue>>>,
}

#[cfg(feature = "has_async")]
impl DemuxedSequencer {
    pub fn new(
        shutdown: Arc<AtomicBool>,
        db_fetcher: Arc<dyn DBFetcher<ParticipantAction>>,
    ) -> Self {
        let (send_move, recieve_move) = unbounded_channel();
        let (send_psbt, recieve_psbt) = unbounded_channel();
        let (send_custom, recieve_custom) = unbounded_channel::<CanonicalJsonValue>();
        DemuxedSequencer {
            sequencer: GenericSequencer::new(shutdown, db_fetcher, Ok),
            send_move,
            recieve_move: Arc::new(Mutex::new(recieve_move)),
            send_psbt,
            recieve_psbt: Arc::new(Mutex::new(recieve_psbt)),
            send_custom,
            recieve_custom: Arc::new(Mutex::new(recieve_custom)),
        }
    }

    #[cfg(feature = "has_async")]
    pub async fn run(self) -> Result<(), JoinError> {
        use std::error::Error;

        spawn(self.sequencer.clone().run());
        spawn({
            let this = self;
            async move {
                let mut listening_moves = true;
                let mut listening_custom = true;
                let mut listening_psbt = true;
                loop {
                    if let Some(e) = this.sequencer.output_move().await {
                        match e.msg() {
                            ParticipantAction::MoveEnvelope(m) if listening_moves => {
                                listening_moves =
                                    this.send_move.send((m.clone(), e.header().key())).is_ok();
                            }
                            ParticipantAction::Custom(c) if listening_custom => {
                                listening_custom = this.send_custom.send(c.clone()).is_ok();
                            }
                            ParticipantAction::PsbtSigningCoordination(c) if listening_psbt => {
                                listening_psbt = this
                                    .send_psbt
                                    .send((c.data.0.clone(), c.channel.clone()))
                                    .is_ok();
                            }
                            _ => {}
                        }
                    }
                    if !(listening_custom || listening_moves || listening_psbt) {
                        break;
                    }
                }
                Ok::<_, Box<dyn Error + Send + Sync>>(())
            }
        });
        Ok(())
    }
}
