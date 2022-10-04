use crate::{Event, TaskSet, OK_T};
use attest_database::connection::MsgDB;
use attest_messages::GenericEnvelope;
use bitcoin::{psbt::PartiallySignedTransaction, XOnlyPublicKey};
use event_log::{
    connection::EventLog,
    db_handle::accessors::{occurrence::ToOccurrence, occurrence_group::OccurrenceGroupID},
};
use game_host_messages::{BroadcastByHost, Channelized};
use game_player_messages::ParticipantAction;
use game_sequencer::{OnlineDBFetcher, UnauthenticatedRawSequencer};
use mine_with_friends_board::{
    game::{GameBoard, MoveRejectReason},
    MoveEnvelope,
};
use std::{
    collections::BTreeMap,
    error::Error,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::{
    spawn,
    sync::{mpsc::UnboundedReceiver, Notify, OwnedMutexGuard},
    task::JoinHandle,
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
    let shutdown: Arc<AtomicBool> = Default::default();
    let db_fetcher = OnlineDBFetcher::new(
        shutdown.clone(),
        Duration::from_secs(1),
        Duration::from_secs(1),
        oracle_key,
        msg_db.clone(),
    );

    let game_setup = get_game_setup(&msg_db, oracle_key).await?;

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
            game_sequencer.run().await;
            OK_T
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
        )
    });
    tasks.push(spawn({
        let recieved_psbt = game_sequencer.recieve_psbt.lock_owned().await;
        async move {
            handle_psbts(
                shutdown.clone(),
                evlog.clone(),
                evlog_group_id,
                recieved_psbt,
                new_synthetic_event,
            )
            .await?;
            OK_T
        }
    }));
    Ok(())
}

pub async fn get_game_setup(
    msg_db: &MsgDB,
    oracle_key: XOnlyPublicKey,
) -> Result<mine_with_friends_board::game::GameSetup, &'static str> {
    let genesis = {
        let handle = msg_db.get_handle().await;
        handle
            .get_message_at_height_for_user::<Channelized<BroadcastByHost>>(oracle_key, 0)
            .map_err(|_e| "Internal Databse Error")?
            .ok_or("No Genesis found for selected Key")?
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
    Ok(game_setup)
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
                    let o: &dyn ToOccurrence = &Event::TransactionFinalized(s, tx);
                    accessor.insert_new_occurrence_now_from(evlog_group_id, o)?;
                    new_synthetic_event.notify_one();
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
) -> JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
    spawn(async move {
        // TODO: Check which game the move is for?
        let mut move_count = 0;
        while let Some((next_move, signed_by)) = moves.recv().await {
            info!(move_ = ?next_move, "New Move Recieved");

            move_count += 1;
            if let Err(MoveRejectReason::GameIsFinished(_)) =
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
                );

                // contract specific, game will not change hereafter
                break;
            }
        }
        Ok(())
    })
}

type HostEnvelope = GenericEnvelope<Channelized<BroadcastByHost>>;
fn make_snapshot(
    move_count: usize,
    evlog: EventLog,
    msg_db: MsgDB,
    oracle_key: XOnlyPublicKey,
    evlog_group_id: OccurrenceGroupID,
) -> JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
    spawn(async move {
        let handle = msg_db.get_handle().await;

        let sequencer_envelopes = handle
            .load_all_messages_for_user_by_key_connected::<_, HostEnvelope>(&oracle_key)
            .map_err(|_| "Database Fetch Error")?;
        let mut m = Default::default();
        handle
            .get_all_messages_collect_into_inconsistent_skip_invalid(&mut None, &mut m, true)
            .map_err(|_| "Database Fetch Error")?;
        drop(handle);

        // todo handle channels...
        let def = Default::default();
        // takes only the first move_count moves, and whittles down the messages to just the ones mentioned.
        let msg_cache = sequencer_envelopes
            .iter()
            .flat_map(|m| match &m.msg().data {
                BroadcastByHost::Sequence(d) => d,
                _ => &def,
            })
            .take(move_count)
            .flat_map(|k| Some((*k, m.remove(k)?)))
            .collect();

        let v = UnauthenticatedRawSequencer::<ParticipantAction> {
            sequencer_envelopes,
            msg_cache,
        };
        if let Ok(v) = serde_json::to_value(v) {
            let accessor = evlog.get_accessor().await;
            accessor
                .insert_new_occurrence_now_from(
                    evlog_group_id,
                    &Event::NewRecompileTriggeringObservation(v),
                )
                .ok();
        }
        OK_T
    })
}
