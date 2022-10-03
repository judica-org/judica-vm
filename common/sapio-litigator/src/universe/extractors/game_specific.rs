use attest_database::connection::MsgDB;
use attest_messages::{AttestEnvelopable, Authenticated, GenericEnvelope};
use bitcoin::XOnlyPublicKey;
use event_log::{connection::EventLog, db_handle::accessors::occurrence_group::OccurrenceGroupID};
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

use crate::{ext::CompiledExt, Event};

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
        gid: OccurrenceGroupID,
        evlog: EventLog,
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
                GenericSequencer<_, (MoveEnvelope, XOnlyPublicKey), (), ParticipantAction>,
            >,
                  output_args_to_try: Sender<(
                UnauthenticatedRawSequencer<ParticipantAction>,
                oneshot::Sender<bool>,
            )>| {
                let msg_db = msg_db.clone();
                let setup = setup.clone();
                let evlog = evlog.clone();
                async move {
                    let mut game = GameBoard::new(&setup);
                    let mut move_count = 0;
                    while let Some((next_move, signed_by)) = recv_new_move.output_move().await {
                        move_count += 1;
                        if let Err(MoveRejectReason::GameIsFinished(_)) =
                            game.play(next_move, signed_by.to_string())
                        {
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

                            let v = UnauthenticatedRawSequencer::<ParticipantAction> {
                                sequencer_envelopes,
                                msg_cache,
                            };
                            if let Ok(v) = serde_json::to_value(v) {
                                let accessor = evlog.get_accessor().await;
                                accessor
                                    .insert_new_occurrence_now_from(
                                        gid,
                                        &Event::NewRecompileTriggeringObservation(v),
                                    )
                                    .ok();
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
