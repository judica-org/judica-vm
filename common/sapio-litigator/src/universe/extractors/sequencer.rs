use attest_database::connection::MsgDB;
use bitcoin::{psbt::PartiallySignedTransaction, XOnlyPublicKey};
use bitcoincore_rpc_async::bitcoin::hashes::hex::ToHex;
use event_log::{
    connection::EventLog,
    db_handle::accessors::{occurrence::ToOccurrence, occurrence_group::OccurrenceGroupID},
};
use game_host_messages::{BroadcastByHost, Channelized};
use game_sequencer::OnlineDBFetcher;
use mine_with_friends_board::{game::GameBoard, MoveEnvelope};
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
use tracing::info;

use crate::Event;

pub async fn sequencer_extractor(
    oracle_key: XOnlyPublicKey,
    msg_db: MsgDB,
    evlog: EventLog,
    evlog_group_id: OccurrenceGroupID,
    new_synthetic_event: Arc<Notify>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let shutdown: Arc<AtomicBool> = Default::default();
    let db_fetcher = OnlineDBFetcher::new(
        shutdown.clone(),
        Duration::from_secs(1),
        Duration::from_secs(1),
        oracle_key,
        msg_db.clone(),
    );

    let genesis = {
        let handle = msg_db.get_handle().await;
        handle
            .get_message_at_height_for_user::<Channelized<BroadcastByHost>>(oracle_key, 0)
            .map_err(|_e| "Internal Databse Error")?
            .ok_or("No Genesis found for selected Key")?
    };
    let game_setup = {
        let m: &Channelized<BroadcastByHost> = genesis.msg();
        match &m.data {
            BroadcastByHost::GameSetup(g) => g,
            _ => return Err("First Message was not a GameSetup".into()),
        }
    };

    let new_game = GameBoard::new(game_setup);

    let game_sequencer =
        game_sequencer::DemuxedSequencer::new(shutdown.clone(), db_fetcher.clone());
    spawn(db_fetcher.run());
    spawn({
        let game_sequencer = game_sequencer.clone();
        game_sequencer.run()
    });

    let _game_task = {
        start_game(
            shutdown.clone(),
            new_game,
            game_sequencer.recieve_move.lock_owned().await,
        )
    };
    let _psbt_task = {
        handle_psbts(
            shutdown.clone(),
            evlog.clone(),
            evlog_group_id,
            game_sequencer.recieve_psbt.lock_owned().await,
            new_synthetic_event,
        )
    };
    Ok(())
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
    mut game: GameBoard,
    mut moves: OwnedMutexGuard<UnboundedReceiver<(MoveEnvelope, XOnlyPublicKey)>>,
) -> JoinHandle<()> {
    spawn(async move {
        // TODO: Check which game the move is for?
        while let Some((game_move, s)) = moves.recv().await {
            info!(move_ = ?game_move, "New Move Recieved");
            game.play(game_move, s.to_hex());
        }
    })
}
