// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    events::{self},
    TaskSet, OK_T,
};
use attest_database::connection::MsgDB;
use attest_messages::{Envelope, GenericEnvelope};
use bitcoin::{psbt::PartiallySignedTransaction, XOnlyPublicKey};
use event_log::{
    connection::EventLog,
    db_handle::accessors::{
        occurrence::sql::insert::Idempotent, occurrence_group::OccurrenceGroupID,
    },
};
use events::TaggedEvent;
use game_host_messages::{BroadcastByHost, Channelized};
use game_player_messages::ParticipantAction;
use game_sequencer::{OnlineDBFetcher, SequencerError, UnauthenticatedRawSequencer};
use mine_with_friends_board::{
    game::{FinishReason, GameBoard, MoveRejectReason},
    MoveEnvelope,
};
use ruma_serde::test::serde_json_eq;
use sapio_base::serialization_helpers::SArc;
use simps::{EventKey, EK_GAME_ACTION_LOSE, EK_GAME_ACTION_WIN};
use std::{
    collections::BTreeMap,
    error::Error,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::{
    spawn,
    sync::{mpsc::UnboundedReceiver, Notify, OwnedMutexGuard},
    task::{spawn_blocking, JoinHandle},
};
use tracing::{debug, info};
pub async fn sequencer_extractor(
    oracle_key: XOnlyPublicKey,
    msg_db: MsgDB,
    evlog: EventLog,
    evlog_group_id: OccurrenceGroupID,
    new_synthetic_event: Arc<Notify>,
    tasks: &TaskSet,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let res = sequencer_extractor_inner(
        oracle_key,
        msg_db,
        evlog,
        evlog_group_id,
        new_synthetic_event,
        tasks,
    )
    .await;
    debug!("Sequencer Extractor Finished Adding Tasks");
    res
}
pub async fn sequencer_extractor_inner(
    oracle_key: XOnlyPublicKey,
    msg_db: MsgDB,
    evlog: EventLog,
    evlog_group_id: OccurrenceGroupID,
    new_synthetic_event: Arc<Notify>,
    tasks: &TaskSet,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let shutdown: Arc<AtomicBool> = Default::default();
    let db_fetcher = OnlineDBFetcher::new(
        shutdown.clone(),
        Duration::from_secs(1),
        Duration::from_secs(1),
        oracle_key,
        msg_db.clone(),
    );

    let (game_setup, contract_setup) = get_game_setup(&msg_db, oracle_key).await?;
    {
        let accessor = evlog.get_accessor().await;
        for s in contract_setup {
            accessor
                .insert_new_occurrence_now_from(evlog_group_id, &s)?
                .map_err(|e: Idempotent| ())
                .ok();
        }
    }

    let new_game = GameBoard::new(&game_setup);

    let game_sequencer =
        game_sequencer::DemuxedSequencer::new(shutdown.clone(), db_fetcher.clone());
    tasks.push(spawn(async move {
        db_fetcher.run().await;
        OK_T
    }));
    tasks.push(spawn({
        let game_sequencer = game_sequencer.clone();
        async move {
            game_sequencer.run().await?;
            Ok(())
        }
    }));

    tasks.push({
        start_game(
            shutdown.clone(),
            evlog.clone(),
            msg_db.clone(),
            oracle_key,
            evlog_group_id,
            new_game,
            game_sequencer.recieve_move.lock_owned().await,
            new_synthetic_event.clone(),
        )
    });
    tasks.push({
        let recieved_psbt = game_sequencer.recieve_psbt.lock_owned().await;
        handle_psbts(
            shutdown.clone(),
            evlog.clone(),
            evlog_group_id,
            recieved_psbt,
            new_synthetic_event,
        )
    });
    Ok(())
}

pub async fn get_game_setup(
    msg_db: &MsgDB,
    oracle_key: XOnlyPublicKey,
) -> Result<(mine_with_friends_board::game::GameSetup, Vec<TaggedEvent>), &'static str> {
    let (genesis, c_setup) = {
        let handle = msg_db.get_handle_read().await;
        let g = spawn_blocking(move || {
            handle.get_message_at_height_for_user::<Channelized<BroadcastByHost>>(oracle_key, 0)
        })
        .await
        .map_err(|_| "Panic in DB")?
        .map_err(|_e| "Internal Databse Error")?
        .ok_or("No Genesis found for selected Key")?;
        let handle = msg_db.get_handle_read().await;
        let c = spawn_blocking(move || {
            handle.get_message_at_height_for_user::<Channelized<BroadcastByHost>>(oracle_key, 1)
        })
        .await
        .map_err(|_| "Panic in DB")?
        .map_err(|_e| "Internal Databse Error")?
        .ok_or("No Setup found for selected Key")?;
        (g, c)
    };
    let game_setup = {
        let m: Channelized<BroadcastByHost> = genesis.inner().into_msg();
        match m.data {
            BroadcastByHost::GameSetup(g) => g,
            _ => {
                debug!(?m.data, "Startup Data Was");
                return Err("First Message was not a GameSetup");
            }
        }
    };
    let contract_setup = {
        let m: Channelized<BroadcastByHost> = c_setup.inner().into_msg();
        match m.data {
            BroadcastByHost::ContractSetup(v) => v
                .into_iter()
                .map(|v| serde_json::from_str(&v.to_string()))
                .collect::<Result<Vec<TaggedEvent>, serde_json::Error>>()
                .map_err(|_| "Could not convert to event")?,
            _ => {
                debug!(?m.data, "Startup Data Was");
                return Err("Second Message was not a ContractSetup");
            }
        }
    };
    Ok((game_setup, contract_setup))
}

pub fn handle_psbts(
    _shutdown: Arc<AtomicBool>,
    evlog: EventLog,
    evlog_group_id: OccurrenceGroupID,
    mut moves: OwnedMutexGuard<UnboundedReceiver<(PartiallySignedTransaction, String)>>,
    new_synthetic_event: Arc<Notify>,
) -> JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
    let mut psbts = BTreeMap::<String, Vec<PartiallySignedTransaction>>::new();
    spawn(async move {
        // TODO: Check which game the move is for?
        let mut psbt_counter = 0;
        while let Some((psbt, s)) = moves.recv().await {
            info!(psbt_id = ?s, "New PSBT Recieved");
            let psbts = psbts.entry(s.clone()).or_default();
            psbts.push(psbt);
            let all_merged =
                psbts
                    .iter()
                    .fold(None, |acc: Option<PartiallySignedTransaction>, new| {
                        let acc_copy = acc.clone();
                        match acc_copy {
                            Some(mut a) => match a.combine(new.clone()) {
                                Ok(()) => Some(a),
                                Err(_) => acc,
                            },
                            None => Some(new.clone()),
                        }
                    });
            if let Some(all_merged) = all_merged {
                let tx = all_merged.extract_tx();
                // TODO: put an actual lookup function here?
                let verified = tx.verify_with_flags(|_o| None, bitcoinconsensus::VERIFY_ALL);
                if verified.is_ok() {
                    let accessor = evlog.get_accessor().await;
                    psbt_counter += 1;
                    let o = events::TaggedEvent(
                        events::Event::TransactionFinalized(s, tx),
                        Some(events::Tag::ScopedCounter("psbts".into(), psbt_counter)),
                    );
                    match accessor.insert_new_occurrence_now_from(evlog_group_id, &o)? {
                        Err(Idempotent::AlreadyExists) => {}
                        Ok(_) => {
                            new_synthetic_event.notify_one();
                        }
                    }
                }
            }
        }
        Ok(())
    })
}
// Play the moves one by one
pub fn start_game(
    _shutdown: Arc<AtomicBool>,
    evlog: EventLog,
    msg_db: MsgDB,
    oracle_key: XOnlyPublicKey,
    evlog_group_id: OccurrenceGroupID,
    mut game: GameBoard,
    mut moves: OwnedMutexGuard<UnboundedReceiver<(MoveEnvelope, XOnlyPublicKey)>>,
    new_synthetic_event: Arc<Notify>,
) -> JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
    spawn(async move {
        // TODO: Check which game the move is for?
        let mut move_count = 0;
        while let Some((next_move, signed_by)) = moves.recv().await {
            info!(move_ = ?next_move, "New Move Recieved");

            move_count += 1;
            if let Err(MoveRejectReason::GameIsFinished(r)) =
                game.play(next_move, signed_by.to_string())
            {
                // todo: get a real one derived only from data we've seen up to the point we've executed.
                // for now, a fresh expensive copy will have to do until we can refactor this.
                make_snapshot(
                    move_count,
                    evlog.clone(),
                    msg_db.clone(),
                    oracle_key,
                    evlog_group_id,
                    match r {
                        FinishReason::TimeExpired => EK_GAME_ACTION_WIN.clone(),
                        FinishReason::DominatingPlayer(_) => EK_GAME_ACTION_LOSE.clone(),
                    },
                    Some(events::Tag::ScopedCounter("game_move".into(), move_count)),
                    new_synthetic_event,
                );

                // contract specific, game will not change hereafter
                break;
            }
        }
        Ok(())
    })
}

type HostEnvelope = GenericEnvelope<Channelized<BroadcastByHost>>;
type ParticipantEnvelope = GenericEnvelope<ParticipantAction>;
fn make_snapshot(
    move_count: u64,
    evlog: EventLog,
    msg_db: MsgDB,
    oracle_key: XOnlyPublicKey,
    evlog_group_id: OccurrenceGroupID,
    event_for: SArc<EventKey>,
    tag: Option<events::Tag>,
    new_synthetic_event: Arc<Notify>,
) -> JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
    spawn(async move {
        let handle = msg_db.get_handle_read().await;
        let (sequencer_envelopes, msg_cache) = spawn_blocking(move || {
            let sequencer_envelopes = handle
                .load_all_messages_for_user_by_key_connected::<_, HostEnvelope>(&oracle_key)
                .map_err(|_| "Database Fetch Error")?;
            let def = Default::default();
            let m: Vec<ParticipantEnvelope> =
                handle
                    .messages_by_hash(sequencer_envelopes.iter().flat_map(
                        |m| match &m.msg().data {
                            BroadcastByHost::Sequence(d) => d,
                            _ => &def,
                        },
                    ))
                    .map_err(|_| "Database Fetch Error")?;
            Ok::<_, &'static str>((
                sequencer_envelopes,
                m.into_iter()
                    .map(|v| (v.canonicalized_hash_ref(), v))
                    .collect(),
            ))
        })
        .await??;

        // takes only the first move_count moves, and whittles down the messages to just the ones mentioned.
        let v = UnauthenticatedRawSequencer::<ParticipantAction> {
            sequencer_envelopes,
            msg_cache,
        };
        if let Ok(v) = serde_json::to_value(v) {
            let accessor = evlog.get_accessor().await;
            // don't care if this fails
            match accessor.insert_new_occurrence_now_from(
                evlog_group_id,
                &events::TaggedEvent(
                    events::Event::NewRecompileTriggeringObservation(v, event_for),
                    tag.clone(),
                ),
            )? {
                Ok(_) => {
                    // TODO: notify?
                    new_synthetic_event.notify_one()
                }
                Err(Idempotent::AlreadyExists) => {}
            }
        }
        OK_T
    })
}
