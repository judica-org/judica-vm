use attest_database::connection::MsgDB;
use attest_messages::{AttestEnvelopable, WrappedJson};
use bitcoin::{psbt::PartiallySignedTransaction, XOnlyPublicKey};
use bitcoincore_rpc_async::bitcoin::hashes::hex::ToHex;
use event_log::{
    connection::EventLog,
    db_handle::accessors::{occurrence::ToOccurrence, occurrence_group::OccurrenceGroupID},
};
use futures::{
    stream::{BoxStream, LocalBoxStream},
    Future, Stream, StreamExt,
};
use game_host_messages::{BroadcastByHost, Channelized};
use game_sequencer::{GenericSequencer, OnlineDBFetcher};
use mine_with_friends_board::{game::GameBoard, MoveEnvelope};
use sapio::contract::{abi::continuation::ContinuationPoint, CompilationError, Compiled};
use sapio_base::{
    effects::{EditableMapEffectDB, PathFragment},
    serialization_helpers::SArc,
    simp::SIMP,
};
use sapio_wasm_plugin::{host::WasmPluginHandle, plugin_handle::PluginHandle, CreateArgs};
use serde_json::Value;
use simps::EventKey;
use std::{
    collections::BTreeMap,
    error::Error,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::{
    spawn,
    sync::{
        mpsc::{channel, Receiver, Sender, UnboundedReceiver},
        Mutex, Notify, OwnedMutexGuard,
    },
    task::JoinHandle,
};
use tracing::info;

use crate::{CompiledExt, Event};

pub trait EvidenceCache<T> {
    fn append(&mut self, item: &T);
    fn retrieve(&self) -> Vec<T>;
}

pub fn attest_stream<T: AttestEnvelopable + 'static>(
    oracle_key: XOnlyPublicKey,
    msg_db: MsgDB,
) -> BoxStream<'static, (<T::Ref as ToOwned>::Owned, XOnlyPublicKey)> {
    let shutdown: Arc<AtomicBool> = Default::default();
    let db_fetcher: Arc<OnlineDBFetcher<T>> = OnlineDBFetcher::new(
        shutdown.clone(),
        Duration::from_secs(1),
        Duration::from_secs(1),
        oracle_key,
        msg_db,
    );
    let sequencer =
        GenericSequencer::<_, _, serde_json::Error, T>::new(shutdown.clone(), db_fetcher, Ok);
    Box::pin(futures::stream::unfold(
        (sequencer, shutdown),
        move |(seq, die)| async move {
            match seq.output_move().await {
                Some(e) => Some(((e.msg().to_owned(), e.header().key().clone()), (seq, die))),
                None => None,
            }
        },
    ))
}

type ModuleInput = <WasmPluginHandle<Compiled> as PluginHandle>::Input;

struct WasmArgsReducer<T: 'static + Send + Sync> {
    input: Sender<T>,
    output: Receiver<(Value, tokio::sync::oneshot::Sender<bool>)>,
}

impl<T: 'static + Send + Sync> WasmArgsReducer<T> {
    async fn new<F, Fut>(mut f: F) -> Self
    where
        F: FnMut(Receiver<T>, Sender<(Value, tokio::sync::oneshot::Sender<bool>)>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let (tx_in, rx_in) = channel(1);
        let (tx_out, rx_out) = channel(1);
        spawn(f(rx_in, tx_out));
        Self {
            input: tx_in,
            output: rx_out,
        }
    }
}
impl WasmArgsReducer<(MoveEnvelope, XOnlyPublicKey)> {
    async fn new_default(init: &CreateArgs<Value>) -> Self {
        let mut args = init.clone();
        Self::new(|recv_new_move, output_args_to_try| async move {
            while let Some(next_move) = recv_new_move.recv().await {
                let (tx_should_save, rx_should_save) = tokio::sync::oneshot::channel();
                output_args_to_try.send((todo!(), tx_should_save)).await;
                if rx_should_save.await {
                    todo!()
                } else {
                    todo!()
                }
            }
        })
        .await
    }
}

async fn module_argument_compilation_attempt_engine<T: 'static + Send + Sync>(
    mut sequenced_moves: BoxStream<'static, T>,
    mut reducer: WasmArgsReducer<T>,
    init: &CreateArgs<Value>,
    handle: Arc<Mutex<WasmPluginHandle<Compiled>>>,
) -> Receiver<Compiled> {
    let (tx, rx) = channel(1);

    let mut args = init.clone();
    spawn(async move {
        let g_handle = handle.lock().await;
        let mut contract = g_handle.call(&PathFragment::Root.into(), &args).ok()?;
        drop(g_handle);
        let game_action = EventKey("action_in_game".into());
        let mut idx = 0;
        while let Some(next_move) = sequenced_moves.next().await {
            if !reducer.input.send(next_move).await.is_ok() {
                break;
            }
            match reducer.output.recv().await {
                Some((new_information_learned_derived_from_reducer, save_arg)) => {
                    let idx_str = SArc(Arc::new(format!("event-{}", idx)));
                    idx += 1;
                    // work on a clone so as to not have an effect if failed
                    let mut new_args = args.clone();
                    let mut save = EditableMapEffectDB::from(new_args.context.effects.clone());

                    for api in contract
                        .continuation_points()
                        .filter(|api| {
                            if let Some(recompiler) =
                                api.simp.get(&simps::EventRecompiler::get_protocol_number())
                            {
                                if let Ok(recompiler) =
                                    simps::EventRecompiler::from_json(recompiler.clone())
                                {
                                    // Only pay attention to events that we are filtering for
                                    if recompiler.filter == game_action {
                                        return true;
                                    }
                                }
                            }
                            false
                        })
                        .filter(|api| {
                            if let Some(schema) = &api.schema {
                                let json_schema = serde_json::to_value(schema.clone())
                                    .expect("RootSchema must always be valid JSON");
                                jsonschema_valid::Config::from_schema(
                                    // since schema is a RootSchema, cannot throw here
                                    &json_schema,
                                    Some(jsonschema_valid::schemas::Draft::Draft6),
                                )
                                .map(|validator| {
                                    validator
                                        .validate(&new_information_learned_derived_from_reducer)
                                        .is_ok()
                                })
                                .unwrap_or(false)
                            } else {
                                false
                            }
                        })
                    {
                        // ensure that if specified, that we skip invalid messages
                        save.effects
                            .entry(SArc(api.path.clone()))
                            .or_default()
                            .insert(
                                idx_str.clone(),
                                new_information_learned_derived_from_reducer.clone(),
                            );
                    }

                    new_args.context.effects = save.into();
                    let g_handle = handle.lock().await;

                    let result: Result<Compiled, CompilationError> =
                        g_handle.call(&PathFragment::Root.into(), &new_args);
                    drop(g_handle);

                    match result {
                        // TODO:  Belt 'n Suspsender Check:
                        // Check that old_state is augmented by new_state
                        Ok(new_contract) => {
                            args = new_args;
                            contract = new_contract;
                            save_arg.send(true);
                            if tx.send(new_contract).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            save_arg.send(false);
                            tracing::debug!(error=?e, "Module did not like the new argument");
                        }
                    }
                }
                None => break,
            }
        }
        None
    });

    rx
}

// pub async fn sequencer_extractor(
//     oracle_key: XOnlyPublicKey,
//     msg_db: MsgDB,
//     evlog: EventLog,
//     evlog_group_id: OccurrenceGroupID,
//     new_synthetic_event: Arc<Notify>,
// ) -> Result<(), Box<dyn Error + Send + Sync>> {
//     let shutdown: Arc<AtomicBool> = Default::default();
//     let db_fetcher = OnlineDBFetcher::new(
//         shutdown.clone(),
//         Duration::from_secs(1),
//         Duration::from_secs(1),
//         oracle_key,
//         msg_db.clone(),
//     );

//     let genesis = {
//         let handle = msg_db.get_handle().await;
//         handle
//             .get_message_at_height_for_user::<Channelized<BroadcastByHost>>(oracle_key, 0)
//             .map_err(|_e| "Internal Databse Error")?
//             .ok_or("No Genesis found for selected Key")?
//     };
//     let game_setup = {
//         let m: &Channelized<BroadcastByHost> = genesis.msg();
//         match &m.data {
//             BroadcastByHost::GameSetup(g) => g,
//             _ => return Err("First Message was not a GameSetup".into()),
//         }
//     };

//     let new_game = GameBoard::new(game_setup);

//     let game_sequencer =
//         game_sequencer::DemuxedSequencer::new(shutdown.clone(), db_fetcher.clone());
//     spawn(db_fetcher.run());
//     spawn({
//         let game_sequencer = game_sequencer.clone();
//         game_sequencer.run()
//     });

//     let _game_task = {
//         start_game(
//             shutdown.clone(),
//             evlog.clone(),
//             evlog_group_id.clone(),
//             new_game,
//             game_sequencer.recieve_move.lock_owned().await,
//         )
//     };
//     let _psbt_task = {
//         handle_psbts(
//             shutdown.clone(),
//             evlog.clone(),
//             evlog_group_id,
//             game_sequencer.recieve_psbt.lock_owned().await,
//             new_synthetic_event,
//         )
//     };
//     Ok(())
// }

// pub fn handle_psbts(
//     _shutdown: Arc<AtomicBool>,
//     evlog: EventLog,
//     evlog_group_id: OccurrenceGroupID,
//     mut moves: OwnedMutexGuard<UnboundedReceiver<(PartiallySignedTransaction, String)>>,
//     new_synthetic_event: Arc<Notify>,
// ) -> JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
//     let mut psbts = BTreeMap::<String, Vec<PartiallySignedTransaction>>::new();
//     spawn(async move {
//         // TODO: Check which game the move is for?
//         while let Some((psbt, s)) = moves.recv().await {
//             info!(psbt_id = ?s, "New PSBT Recieved");
//             let psbts = psbts.entry(s.clone()).or_default();
//             psbts.push(psbt);
//             let all_merged =
//                 psbts
//                     .iter()
//                     .fold(None, |acc: Option<PartiallySignedTransaction>, new| {
//                         let acc_copy = acc.clone();
//                         match acc_copy {
//                             Some(mut a) => match a.combine(new.clone()) {
//                                 Ok(()) => Some(a),
//                                 Err(_) => acc,
//                             },
//                             None => Some(new.clone()),
//                         }
//                     });
//             if let Some(all_merged) = all_merged {
//                 let tx = all_merged.extract_tx();
//                 // TODO: put an actual lookup function here?
//                 let verified = tx.verify_with_flags(|_o| None, bitcoinconsensus::VERIFY_ALL);
//                 if verified.is_ok() {
//                     let accessor = evlog.get_accessor().await;
//                     let o: &dyn ToOccurrence = &Event::TransactionFinalized(s, tx);
//                     accessor.insert_new_occurrence_now_from(evlog_group_id, o)?;
//                     new_synthetic_event.notify_one();
//                 }
//             }
//         }
//         Ok(())
//     })
// }
// // Play the moves one by one
// pub fn start_game(
//     _shutdown: Arc<AtomicBool>,
//     evlog: EventLog,
//     evlog_group_id: OccurrenceGroupID,
//     mut game: GameBoard,
//     mut moves: OwnedMutexGuard<UnboundedReceiver<(MoveEnvelope, XOnlyPublicKey)>>,
// ) -> JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
//     spawn(async move {
//         // TODO: Check which game the move is for?
//         while let Some((game_move, s)) = moves.recv().await {
//             info!(move_ = ?game_move, "New Move Recieved");
//             match game.play(game_move, s.to_hex()) {
//                 Ok(()) => {
//                     let accessor = evlog.get_accessor().await;
//                     let o: &dyn ToOccurrence =
//                         &Event::ProtocolMessage((ruma_serde::to_canonical_value(game_move)?, s));
//                     accessor.insert_new_occurrence_now_from(evlog_group_id, o)?;
//                 }
//                 Err(()) => {
//                     todo!("Handle Invalid Moves in Attest Sequence Extractor")
//                 }
//             }
//         }
//         Ok(())
//     })
// }
