//! Module Game Init defineds HTTP Handlers for registering and creating a new game.
//!
//! It contains hacks nescessary for an MVP.
//!
//! Basic Flow is:
//!
//! create_new_game_instance() -> NewGame{password: JoinCode, join: JoinCode}
//!
//! for each player:
//!     add_player(join, genesis envelope) -> ()
//!
//! finish_setup(password, join, ... params) -> CreatedNewChain {genesis: Envelope, name: String}

use crate::{
    app::{create_new_attestation_chain, CreatedNewChain},
    globals::Globals,
};
use attest_database::connection::MsgDB;
use attest_messages::{AuthenticationError, GenericEnvelope};
use axum::{
    http::{Response, StatusCode},
    Extension, Json,
};
use bitcoincore_rpc_async::{json::WalletCreateFundedPsbtOptions, RpcApi};
use event_log::db_handle::accessors::{
    occurrence::sql::Idempotent,
};
use game_host_messages::{AddPlayerError, FinishArgs, JoinCode, NewGame, NewGameArgs};
use game_player_messages::ParticipantAction;
use mine_with_friends_board::{
    game::{game_move::GameMove, GameSetup},
    sanitize::Unsanitized,
    MoveEnvelope,
};
use sapio::sapio_base::effects::{EffectPath, PathFragment};
use sapio_bitcoin::{
    psbt::PartiallySignedTransaction,
    secp256k1::{All, Secp256k1},
    Address, Script,
};
use sapio_litigator_events::{Event, ModuleRepo, Tag, TaggedEvent};
use serde::Deserialize;
use tracing::{debug, trace};

use std::{
    collections::{HashMap, VecDeque},
    str::FromStr,
    sync::{Arc, Weak},
    time::Duration,
};
use tokio::{sync::Mutex, task::spawn_blocking};

struct Metadata {
    state: Mutex<GameStartingState>,
    code: Arc<JoinCode>,
    admin: JoinCode,
    duration: Duration,
}

type MetadataRc = Arc<Metadata>;
type MetadataWRc = Weak<Metadata>;
enum GameStartingState {
    AddingPlayers(Vec<GenericEnvelope<ParticipantAction>>),
    WaitingForSetup(Vec<GenericEnvelope<ParticipantAction>>),
    Setup(Vec<GenericEnvelope<ParticipantAction>>, GameSetup),
}
pub struct NewGameDB {
    states: HashMap<Arc<JoinCode>, MetadataRc>,
    by_time: VecDeque<MetadataWRc>,
}

impl NewGameDB {
    pub fn new() -> NewGameDB {
        Self {
            states: HashMap::with_capacity(1000),
            by_time: VecDeque::with_capacity(1000),
        }
    }
}

impl NewGameDB {
    fn add_new_game(&mut self, duration: Duration) -> (JoinCode, Arc<JoinCode>) {
        let m = Arc::new(Metadata {
            state: Mutex::new(GameStartingState::new()),
            code: Arc::new(Default::default()),
            admin: Default::default(),
            duration,
        });
        let code = (m.admin, m.code.clone());

        if self.by_time.capacity() == self.by_time.len() {
            if let Some(remove) = self.by_time.pop_front() {
                if let Some(remove) = remove.upgrade() {
                    self.states.remove(&remove.code);
                }
            }
        }

        self.by_time.push_back(Arc::downgrade(&m));
        self.states.insert(m.code.clone(), m);
        code
    }
}
const MAX_PLAYERS: usize = 10;

impl GameStartingState {
    fn new() -> GameStartingState {
        GameStartingState::AddingPlayers(vec![])
    }
    fn add_player(&mut self, p: GenericEnvelope<ParticipantAction>) -> Result<(), AddPlayerError> {
        if p.get_genesis_hash() != p.canonicalized_hash_ref() {
            return Err(AddPlayerError::NotGenesisEnvelope);
        }
        match p.msg() {
            ParticipantAction::MoveEnvelope(MoveEnvelope {
                d: Unsanitized(GameMove::Heartbeat(_)),
                sequence: _,
                time_millis: _,
            }) => {}
            _ => return Err(AddPlayerError::WrongFirstMessage),
        };
        match self {
            GameStartingState::AddingPlayers(ref mut v) => {
                v.push(p);
                if v.len() == MAX_PLAYERS as usize {
                    let mut clr = vec![];
                    std::mem::swap(&mut clr, v);
                    *self = GameStartingState::WaitingForSetup(clr);
                    Ok(())
                } else {
                    Ok(())
                }
            }
            GameStartingState::Setup(_, _) => Err(AddPlayerError::AlreadySetup),
            GameStartingState::WaitingForSetup(_) => Err(AddPlayerError::NoMorePlayers),
        }
    }
    fn finalize_setup(
        &mut self,
        finish_time: Duration,
        start_amount: u64,
    ) -> Result<(), AddPlayerError> {
        match self {
            GameStartingState::AddingPlayers(v) | GameStartingState::WaitingForSetup(v) => {
                let players = v.iter().map(|i| i.header().key().to_string()).collect();
                let game = GameSetup {
                    players,
                    start_amount,
                    // TODO: maybe bounds check? But should be safe from before
                    finish_time: finish_time.as_millis() as u64,
                };

                let mut clr = vec![];
                std::mem::swap(&mut clr, v);
                *self = GameStartingState::Setup(clr, game);
                Ok(())
            }
            GameStartingState::Setup(_, _) => Err(AddPlayerError::AlreadySetup),
        }
    }
}

pub async fn create_new_game_instance(
    Json(minutes): Json<NewGameArgs>,
    Extension(db): Extension<Arc<Mutex<NewGameDB>>>,
) -> Result<(Response<()>, Json<NewGame>), (StatusCode, &'static str)> {
    if minutes.duration_minutes > 300 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Duration too long, must be <= 300 minutes",
        ));
    }
    let code = db
        .lock()
        .await
        .add_new_game(Duration::from_secs(minutes.duration_minutes as u64 * 60));
    let new = NewGame {
        password: code.0,
        join: *code.1,
    };
    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(new),
    ))
}

pub async fn add_player(
    Json((code, envelope)): Json<(JoinCode, GenericEnvelope<ParticipantAction>)>,
    Extension(db): Extension<Arc<Mutex<NewGameDB>>>,
) -> Result<(Response<()>, Json<()>), (StatusCode, String)> {
    if let Some(v) = db.lock().await.states.get(&code) {
        v.state
            .lock()
            .await
            .add_player(envelope)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok((
        Response::builder()
            .status(200)
            .header("Access-Control-Allow-Origin", "*")
            .body(())
            .expect("Response<()> should always be valid"),
        Json(()),
    ))
}

pub async fn finish_setup(
    msgdb: Extension<MsgDB>,
    secp: Extension<Arc<Secp256k1<All>>>,
    Json(FinishArgs {
        passcode,
        code,
        start_amount,
    }): Json<FinishArgs>,
    Extension(db): Extension<Arc<Mutex<NewGameDB>>>,
    // TODO: Add these to the layer in app.rs / move to globals?
    Extension(globals): Extension<Globals>,
) -> Result<(Response<()>, Json<CreatedNewChain>), (StatusCode, String)> {
    let v = db
        .lock()
        .await
        .states
        .get(&code)
        .ok_or((StatusCode::NOT_FOUND, "No Such Game".into()))?
        .clone();

    trace!(game_id=?code, "Starting to Create Game");
    if passcode != v.admin {
        return Err((StatusCode::UNAUTHORIZED, "Wrong Passcode".into()));
    }
    trace!(game_id=?code, "Passcode Matched");
    let mut game = v.state.lock().await;
    game.finalize_setup(v.duration, start_amount)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    match &*game {
        GameStartingState::AddingPlayers(_) | GameStartingState::WaitingForSetup(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Game Not in Correct State".into(),
        )),
        GameStartingState::Setup(envelopes, gs) => {
            let authed: Vec<_> = envelopes
                .iter()
                .map(|e| e.self_authenticate(&secp.0))
                .collect::<Result<_, AuthenticationError>>()
                .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

            trace!(game_id=?code, "Envelopes Valid");
            for (i, env) in authed.into_iter().enumerate() {
                let mut handle = msgdb.get_handle_all().await;
                spawn_blocking(move || {
                    handle.insert_user_by_genesis_envelope(
                        format!("{}::{}", String::from(code), i),
                        env,
                    )
                })
                .await
                .map_err(|_e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal Database Panic".to_string(),
                    )
                })?
                .map_err(|_e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Database Error".to_string(),
                    )
                })?
                // These errors are OK here
                .ok();
            }
            trace!(game_id=?code, "Envelopes Inserted");
            let resp = create_new_attestation_chain(
                Json((
                    envelopes
                        .iter()
                        .map(|m| m.canonicalized_hash_ref())
                        .collect(),
                    gs.clone(),
                )),
                msgdb,
                secp,
            )
            .await
            .map_err(|e| (e.0, e.1.to_owned()));

            trace!(game_id=?code, "Genesis Created");
            if let Ok((_, Json(ref b))) = resp {
                let args = sapio_litigator_events::convert_setup_to_contract_args(
                    gs.to_owned(),
                    &b.sequencer_key,
                )
                .map_err(|_e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Error Creating Sapio Args".to_string(),
                    )
                })?;

                trace!(game_id=?code, ?args, "Contract Args Made");
                let compiled = {
                    let module = globals.compiler_module.lock().await;
                    module
                        .call(&EffectPath::from(PathFragment::Root), &args)
                        .map_err(|e| {
                            debug!(game_id=?code, ?e, "Contract Object Failed Compiled");
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Error Compiling Sapio Contract".to_string(),
                            )
                        })?
                };
                trace!(game_id=?code, "Contract Object Compiled");
                let address =
                    Address::from_script(&Script::from(compiled.address), globals.bitcoin_network)
                        .ok_or((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Error Converting Into Address".to_string(),
                        ))?;

                debug!(game_id=?code, ?address, "Contract Address");
                let amount = compiled.amount_range.max();
                let psbt = globals
                    .bitcoin_rpc
                    .wallet_create_funded_psbt(
                        &[],
                        &HashMap::from_iter([(address.to_string(), amount)].into_iter()),
                        None,
                        Some(WalletCreateFundedPsbtOptions {
                            change_address: None,
                            change_position: Some(1),
                            change_type: None,
                            include_watching: None,
                            lock_unspent: Some(true),
                            fee_rate: None,
                            subtract_fee_from_outputs: vec![],
                            replaceable: Some(true),
                            conf_target: Some(1),
                            estimate_mode: None,
                        }),
                        Some(true),
                    )
                    .await
                    .map_err(|_| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Error Making PSBTs".to_string(),
                        )
                    })?;
                #[derive(Deserialize, Debug)]
                struct R {
                    psbt: String,
                    complete: bool,
                }
                let r = globals
                    .bitcoin_rpc
                    .call::<R>("walletprocesspsbt", &[serde_json::Value::String(psbt.psbt)])
                    .await
                    .map_err(|_| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Error Signing PSBTs".to_string(),
                        )
                    })?;
                trace!(game_id=?code, ?r, "Got PSBT");
                if !r.complete {
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, "PSBT NOT DONE".into()));
                }

                let psbt = PartiallySignedTransaction::from_str(&r.psbt).map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "PSBT Invalid".to_string(),
                    )
                })?;

                let tx = psbt.extract_tx();

                let seq_str = b.sequencer_key.to_string();
                let tx_evt = TaggedEvent(
                    Event::TransactionFinalized("default".into(), tx.clone()),
                    Some(Tag::ScopedValue(seq_str.clone(), "funding_tx".into())),
                );
                {
                    let accessor = globals.evlog.get_accessor().await;
                    // TODO: SPAWN_BLOCKING
                    let sequencer_group = accessor
                        .insert_new_occurrence_group(&seq_str)
                        .or_else(|_| accessor.get_occurrence_group_by_key(&seq_str))
                        .map_err(|_| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Could not get group by key".into(),
                            )
                        })?;
                    trace!(game_id=?code, ?sequencer_group, "Got Sequencer Group");
                    accessor
                        .insert_new_occurrence_now_from(
                            sequencer_group,
                            &TaggedEvent(
                                Event::ModuleBytes(
                                    ModuleRepo::default_group_key(),
                                    globals.module_tag.clone(),
                                ),
                                Some(Tag::InitModule),
                            ),
                        )
                        .map_err(|_| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Could not Insert new Group".into(),
                            )
                        })?
                        // ignore idempotent error, safe if the library ever changes
                        .map_err(|_: Idempotent| ())
                        .ok();

                    trace!(game_id=?code, "Inserted ModuleBytes into evlog");
                    accessor
                        .insert_new_occurrence_now_from(
                            sequencer_group,
                            &TaggedEvent(Event::CreateArgs(args), Some(Tag::CreateArgs)),
                        )
                        .map_err(|_| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Could not insert create args".into(),
                            )
                        })?
                        .map_err(|_| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Idempotent Key already inserted".into(),
                            )
                        })?;

                    trace!(game_id=?code, "Inserted CreateArgs into evlog");
                    accessor
                        .insert_new_occurrence_now_from(
                            sequencer_group,
                            &TaggedEvent(
                                Event::Rebind(sapio_bitcoin::OutPoint {
                                    txid: tx.txid(),
                                    vout: 0,
                                }),
                                Some(Tag::FirstBind),
                            ),
                        )
                        .map_err(|_| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Could not insert rebind args".into(),
                            )
                        })?
                        .map_err(|_| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Idempotent Key already inserted".into(),
                            )
                        })?;
                    trace!(game_id=?code, "Inserted Rebind into evlog");
                    accessor
                        .insert_new_occurrence_now_from(sequencer_group, &tx_evt)
                        .map_err(|_| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Could not insert tx evt".into(),
                            )
                        })?
                        .ok();
                    trace!(game_id=?code, "Inserted PSBT into evlog");
                    // lastly send the tx...
                    globals.bitcoin_rpc.send_raw_transaction(&tx).await.ok();
                }

                resp
            } else {
                resp
            }
        }
    }
}
