use attest_database::connection::MsgDB;
use attest_messages::{AttestEnvelopable, Authenticated, GenericEnvelope};
use bitcoin::XOnlyPublicKey;
use futures::Future;
use game_host_messages::{BroadcastByHost, Channelized};
use game_player_messages::ParticipantAction;
use game_sequencer::{GenericSequencer, OnlineDBFetcher, UnauthenticatedRawSequencer};
use mine_with_friends_board::{
    game::{GameBoard, GameSetup, MoveRejectReason},
    MoveEnvelope,
};
use sapio::contract::Compiled;
use sapio_base::{
    effects::{EditableMapEffectDB, PathFragment},
    serialization_helpers::SArc,
    simp::SIMP,
};
use sapio_wasm_plugin::{host::WasmPluginHandle, plugin_handle::PluginHandle, CreateArgs};
use serde_json::Value;
use simps::EventKey;
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::{
    spawn,
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot, Mutex,
    },
};

use crate::CompiledExt;

pub fn attest_stream<F, R, E, M>(
    oracle_key: XOnlyPublicKey,
    msg_db: MsgDB,
    move_read_fn: F,
) -> Arc<GenericSequencer<F, R, E, M>>
where
    F: Fn(Authenticated<GenericEnvelope<M>>) -> Result<R, E> + Send + Sync + 'static,
    E: Sync + Send + 'static + std::fmt::Debug,
    R: Send + 'static,
    M: AttestEnvelopable + 'static,
{
    let shutdown: Arc<AtomicBool> = Default::default();
    let db_fetcher: Arc<OnlineDBFetcher<M>> = OnlineDBFetcher::new(
        shutdown.clone(),
        Duration::from_secs(1),
        Duration::from_secs(1),
        oracle_key,
        msg_db,
    );
    let sequencer =
        GenericSequencer::<_, _, _, _>::new(shutdown.clone(), db_fetcher.clone(), move_read_fn);
    spawn(db_fetcher.run());
    let s = sequencer.clone();
    spawn(s.run());
    sequencer
}

pub struct WasmArgsReducer<Output> {
    output: Receiver<(Output, tokio::sync::oneshot::Sender<bool>)>,
}

impl<Output> WasmArgsReducer<Output> {
    pub fn new<F, R, E, M, Spawn, Fut, FutOut>(
        mut f: Spawn,
        g: Arc<GenericSequencer<F, R, E, M>>,
    ) -> Self
    where
        Spawn: FnMut(
            Arc<GenericSequencer<F, R, E, M>>,
            Sender<(Output, tokio::sync::oneshot::Sender<bool>)>,
        ) -> Fut,
        Fut: Future<Output = FutOut> + Send + 'static,
        FutOut: Send + Sync + 'static,
        F: Fn(Authenticated<GenericEnvelope<M>>) -> Result<R, E> + Send + Sync + 'static,
        E: Sync + Send + 'static + std::fmt::Debug,
        R: Send + 'static,
        M: AttestEnvelopable + 'static,
    {
        let (tx_out, rx_out) = channel(1);
        spawn(f(g, tx_out));
        Self { output: rx_out }
    }
}
impl WasmArgsReducer<UnauthenticatedRawSequencer<ParticipantAction>> {
    pub async fn new_default<F>(
        msg_db: MsgDB,
        key: XOnlyPublicKey,
        g: Arc<GenericSequencer<F, (MoveEnvelope, XOnlyPublicKey), (), ParticipantAction>>,
    ) -> Result<(Self, Arc<GameSetup>), &'static str>
    where
        F: Fn(
                Authenticated<GenericEnvelope<ParticipantAction>>,
            ) -> Result<(MoveEnvelope, XOnlyPublicKey), ()>
            + Send
            + Sync
            + 'static,
    {
        let first = msg_db
            .get_handle()
            .await
            .get_message_at_height_for_user::<Channelized<BroadcastByHost>>(key, 0)
            .map_err(|_| "DB Error")?
            .ok_or("Not Found")?;
        let setup = Arc::new(
            match &first.msg().data {
                BroadcastByHost::GameSetup(g) => g,
                BroadcastByHost::Sequence(_)
                | BroadcastByHost::NewPeer(_)
                | BroadcastByHost::Heartbeat => return Err("Wrong first message"),
            }
            .clone(),
        );
        let setup_ret = setup.clone();
        let task = Self::new(
            move |mut recv_new_move: Arc<
                GenericSequencer<
                    _,
                    (MoveEnvelope, XOnlyPublicKey),
                    (),
                    ParticipantAction,
                >,
            >,
                  output_args_to_try: Sender<(
                UnauthenticatedRawSequencer<ParticipantAction>,
                oneshot::Sender<bool>,
            )>| {
                let msg_db = msg_db.clone();
                let setup = setup.clone();
                async move {
                    let mut game = GameBoard::new(&setup);
                    let mut move_count = 0;
                    while let Some((next_move, signed_by)) = recv_new_move.output_move().await {
                        move_count += 1;
                        if let Err(MoveRejectReason::GameIsFinished(_)) =
                            game.play(next_move, signed_by.to_string())
                        {
                            let (tx_should_quit, rx_should_quit) = tokio::sync::oneshot::channel();
                            // todo: get a real one derived only from data we've seen up to the point we've executed.
                            // for now, a fresh expensive copy will have to do until we can refactor this.
                            let handle = msg_db.get_handle().await;

                            let sequencer_envelopes=  handle.load_all_messages_for_user_by_key_connected::<_, GenericEnvelope<Channelized<BroadcastByHost>>>(&key)
                            .map_err(|_| "Database Fetch Error")?;
                            let mut m = Default::default();
                            handle
                                .get_all_messages_collect_into_inconsistent_skip_invalid(
                                    &mut None, &mut m, true,
                                )
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

                            let v = UnauthenticatedRawSequencer {
                                sequencer_envelopes,
                                msg_cache,
                            };
                            if output_args_to_try.send((v, tx_should_quit)).await.is_err() {
                                break;
                            }
                            match rx_should_quit.await {
                                Ok(true) => {
                                    break;
                                }
                                Ok(false) => {
                                    continue;
                                }
                                Err(_) => break,
                            }
                        }
                    }
                    Ok::<(), &'static str>(())
                }
            },
            g,
        );
        Ok((task, setup_ret))
    }
}

pub async fn module_argument_compilation_attempt_engine(
    mut reducer: WasmArgsReducer<UnauthenticatedRawSequencer<ParticipantAction>>,
    init: &CreateArgs<Value>,
    handle: Arc<Mutex<WasmPluginHandle<Compiled>>>,
) -> Receiver<Compiled> {
    let (tx_contract_state, rx) = channel(1);

    let mut args = init.clone();
    spawn(async move {
        let mut contract = {
            let g_handle = handle.lock().await;
            g_handle.call(&PathFragment::Root.into(), &args).ok()?
        };
        let game_action = EventKey("action_in_game".into());
        let mut idx = 0;

        while let Some((new_information_learned_derived_from_reducer, quit_reducer)) =
            reducer.output.recv().await
        {
            let idx_str = SArc(Arc::new(format!("event-{}", idx)));
            idx += 1;
            // work on a clone so as to not have an effect if failed
            let mut new_args = args.clone();
            let mut save = EditableMapEffectDB::from(new_args.context.effects.clone());

            let new_info_as_v = if let Ok(new_info_as_v) =
                serde_json::to_value(&new_information_learned_derived_from_reducer)
            {
                new_info_as_v
            } else {
                tracing::warn!("Could Not Serialize UnauthenticatedRawSequencer");
                continue;
            };
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
                        let json_schema = serde_json::to_value(&schema)
                            .expect("RootSchema must always be valid JSON");
                        // todo: cache?
                        jsonschema_valid::Config::from_schema(
                            // since schema is a RootSchema, cannot throw here
                            &json_schema,
                            Some(jsonschema_valid::schemas::Draft::Draft6),
                        )
                        .map(|validator| validator.validate(&new_info_as_v).is_ok())
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
                        // todo: maybe use arcs here too
                        new_info_as_v.clone(),
                    );
            }

            new_args.context.effects = save.into();
            let result: Result<Compiled, ()> = {
                let g_handle = handle.lock().await;
                // drop error before releasing g_handle so that the CompilationError non-send type
                // doesn't get held across an await point
                g_handle
                    .call(&PathFragment::Root.into(), &new_args)
                    .map_err(|e| tracing::debug!(error=?e, "Module did not like the new argument"))
            };

            match result {
                // TODO:  Belt 'n Suspsender Check:
                // Check that old_state is augmented by new_state
                Ok(new_contract) => {
                    args = new_args;
                    contract = new_contract;
                    quit_reducer.send(true);
                    if tx_contract_state.send(contract.clone()).await.is_err() {
                        // Channel Closure
                        break;
                    }
                }
                Err(e) => {
                    quit_reducer.send(false);
                }
            }
        }
        None::<()>
    });

    rx
}
