use attest_messages::{Authenticated, GenericEnvelope};
use bitcoin::blockdata::script::Script;
use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::{Amount, OutPoint, Transaction, Txid, XOnlyPublicKey};
use emulator_connect::{CTVAvailable, CTVEmulator};
use event_log::db_handle::accessors::occurrence::{ApplicationTypeID, ToOccurrence};
use ext::CompiledExt;
use futures::select;
use futures::stream::FuturesUnordered;
use game_player_messages::ParticipantAction;
use mine_with_friends_board::MoveEnvelope;
use ruma_serde::CanonicalJsonValue;
use sapio::contract::object::SapioStudioFormat;
use sapio::contract::Compiled;
use sapio::util::amountrange::AmountF64;
use sapio_base::effects::{EditableMapEffectDB, PathFragment};
use sapio_base::serialization_helpers::SArc;
use sapio_base::simp::SIMP;
use sapio_base::txindex::TxIndexLogger;
use sapio_wasm_plugin::host::{plugin_handle::ModuleLocator, WasmPluginHandle};
use sapio_wasm_plugin::plugin_handle::PluginHandle;
use sapio_wasm_plugin::{ContextualArguments, CreateArgs};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use simps::{AutoBroadcast, EventKey, GameKernel, PK};
use std::collections::BTreeMap;
use std::error::Error;
use std::rc::Rc;
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::{Mutex, Notify};
use tracing::{debug, info, trace};
use universe::extractors::sequencer::get_game_setup;
mod universe;

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    ModuleBytes(Vec<u8>),
    TransactionFinalized(String, Transaction),
    Rebind(OutPoint),
    SyntheticPeriodicActions(i64),
    NewRecompileTriggeringObservation(Value),
}

impl ToOccurrence for Event {
    fn to_data(&self) -> CanonicalJsonValue {
        ruma_serde::to_canonical_value(self).unwrap()
    }
    fn stable_typeid(&self) -> ApplicationTypeID {
        ApplicationTypeID::from_inner("LitigatorEvent")
    }
}

struct AppState {
    bound_to: Option<OutPoint>,
    psbt_db: Arc<PSBTDatabase>,
    event_counter: u64,
    // Initialized after first move
    module: Arc<Mutex<Result<WasmPluginHandle<Compiled>, String>>>,
    args: Result<CreateArgs<Value>, String>,
    contract: Result<Compiled, String>,
}

pub mod ext;

struct PSBTDatabase {
    cache: BTreeMap<Txid, Vec<PartiallySignedTransaction>>,
    signing_services: Vec<String>,
}

impl PSBTDatabase {
    fn new() -> Self {
        PSBTDatabase {
            cache: Default::default(),
            signing_services: vec![],
        }
    }
    fn try_signing(&mut self, _psbt: PartiallySignedTransaction) -> Option<Transaction> {
        // logic here should be to either try pinging services and attempt to finalize a txn out of it
        todo!()
    }
}

pub mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let mut args = std::env::args();
    let config = config::Config::from_env()?;
    match args.nth(1) {
        None => Err("Must have init or run")?,
        Some(s) if &s == "init" => Ok(init_contract_event_log(config).await?),
        Some(s) if &s == "run" => Ok(litigate_contract(config).await?),
        Some(s) => {
            println!("Invalid argument: {}", s);
            Ok(())
        }
    }
}

async fn init_contract_event_log(config: config::Config) -> Result<(), Box<dyn std::error::Error>> {
    let evlog = config.get_event_log().await?;
    let accessor = evlog.get_accessor().await;
    let group = config.event_log.group.clone();
    let gid = match accessor.get_occurrence_group_by_key(&group) {
        Ok(gid) => {
            info!("Instance {} has already been initialized", group);
            gid
        }
        Err(_) => {
            info!("Initializing OccurenceGroupID for {:?}", group);
            let gid = accessor.insert_new_occurrence_group(&group)?;
            gid
        }
    };
    let occurrences = accessor.get_occurrences_for_group(gid)?;
    if occurrences.is_empty() {
        info!(
            "Initializing OccurrenceGroup with module at {:?}",
            config.contract_location
        );
        let location = config.contract_location.to_str().unwrap();
        insert_init(location, &config, &accessor, gid).await?;
    }
    Ok(())
}

async fn insert_init(
    location: &str,
    config: &config::Config,
    accessor: &event_log::db_handle::EventLogAccessor<'_>,
    gid: event_log::db_handle::accessors::occurrence_group::OccurrenceGroupID,
) -> Result<(), Box<dyn Error>> {
    let ev = Event::ModuleBytes(tokio::fs::read(&location).await?);
    accessor.insert_new_occurrence_now_from(gid, &ev)?;
    Ok(())
}

async fn litigate_contract(config: config::Config) -> Result<(), Box<dyn std::error::Error>> {
    // initialize db connection to the event log
    let evlog = config.get_event_log().await?;
    let msg_db = config.get_db().await?;
    let _bitcoin = config.get_bitcoin_rpc().await?;

    // get location of project directory modules
    let data_dir = config::data_dir_modules(&config.app_instance);

    // allocate a CTV Emulator
    let emulator: Arc<dyn CTVEmulator> = Arc::new(CTVAvailable);

    // create wasm plugin handle from contract initialization data at beginning of the event log
    let ogid = evlog
        .get_accessor()
        .await
        .get_occurrence_group_by_key(&config.event_log.group)?;

    // check which continuation points need attest message routing

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let key = &config.event_log.group;
    let evlog_group_id = {
        let accessor = evlog.get_accessor().await;
        let evlog_group_id = accessor.get_occurrence_group_by_key(key);
        let evlog_group_id =
            evlog_group_id.or_else(|_| accessor.insert_new_occurrence_group(key))?;
        evlog_group_id
    };
    // This should be in notify_one mode, which means that only the db reader
    // should be calling notified and wakers should call notify_one.
    let new_synthetic_event = Arc::new(Notify::new());
    let config = Arc::new(config);

    let tasks = FuturesUnordered::new();
    let extractors = spawn(start_extractors(
        evlog.clone(),
        evlog_group_id,
        tx,
        new_synthetic_event,
        config.clone(),
        msg_db.clone(),
    ));

    let root = PathFragment::Root.into();
    let mut state = AppState {
        args: Err("No Args Loaded".into()),
        module: Arc::new(Mutex::new(Err("No Module Loaded".into()))),
        contract: Err("No Compiled Object".into()),
        bound_to: None,
        psbt_db: Arc::new(PSBTDatabase::new()),
        event_counter: 0,
    };

    let game_action = EventKey("action_in_game".into());
    let evl = spawn(event_loop(
        rx,
        state,
        emulator,
        data_dir,
        msg_db,
        config,
        root,
        game_action,
    ));

    tasks.push(extractors);
    tasks.push(evl);

    for task in tasks {
        let r = task.await?;
        trace!(?r);
        r.map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn start_extractors(
    evlog: event_log::connection::EventLog,
    evlog_group_id: event_log::db_handle::accessors::occurrence_group::OccurrenceGroupID,
    tx: tokio::sync::mpsc::Sender<Event>,
    new_synthetic_event: Arc<Notify>,
    config: Arc<config::Config>,
    msg_db: attest_database::connection::MsgDB,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let tasks = FuturesUnordered::new();
    tasks.push(tokio::spawn(universe::linearized::event_log_processor(
        evlog.clone(),
        evlog_group_id,
        tx,
        new_synthetic_event.clone(),
    )));
    tasks.push(tokio::spawn(
        universe::extractors::time::time_event_extractor(
            evlog.clone(),
            evlog_group_id,
            new_synthetic_event.clone(),
        ),
    ));
    tasks.push(tokio::spawn(
        universe::extractors::sequencer::sequencer_extractor(
            config.oracle_key,
            msg_db.clone(),
            evlog.clone(),
            evlog_group_id,
            new_synthetic_event.clone(),
        ),
    ));
    if let Some(task) = tasks.into_iter().next() {
        task.await?
    } else {
        Ok(())
    }
}

async fn event_loop(
    mut rx: tokio::sync::mpsc::Receiver<Event>,
    mut state: AppState,
    emulator: Arc<dyn CTVEmulator>,
    data_dir: std::path::PathBuf,
    msg_db: attest_database::connection::MsgDB,
    config: Arc<config::Config>,
    root: sapio_base::reverse_path::ReversePath<PathFragment>,
    game_action: EventKey,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    Ok(loop {
        match rx.recv().await {
            Some(Event::TransactionFinalized(_s, _tx)) => {
                info!("Transaction Finalized");
            }
            Some(Event::SyntheticPeriodicActions(time)) => {
                info!(time, "SyntehticPeriodicActions");
                if let Some(out) = state.bound_to.as_ref() {
                    let c = &state.contract.as_ref().map_err(|e| e.as_str())?;
                    if let Ok(program) = c.bind_psbt(
                        *out,
                        Default::default(),
                        Rc::new(TxIndexLogger::new()),
                        emulator.as_ref(),
                    ) {
                        for obj in program.program.values() {
                            for tx in obj.txs.iter() {
                                let SapioStudioFormat::LinkedPSBT {
                                    psbt: _,
                                    hex: _,
                                    metadata,
                                    output_metadata: _,
                                    added_output_metadata: _,
                                } = tx;
                                if let Some(_data) =
                                    metadata.simp.get(&AutoBroadcast::get_protocol_number())
                                {
                                    // TODO:
                                    // - Send PSBT out for signatures?
                                }
                            }
                        }
                    }
                }
            }
            Some(Event::ModuleBytes(contract_bytes)) => {
                info!("ModuleBytes");
                let locator: ModuleLocator = ModuleLocator::Bytes(contract_bytes);
                let module = WasmPluginHandle::<Compiled>::new_async(
                    &data_dir,
                    &emulator,
                    locator,
                    bitcoin::Network::Bitcoin,
                    Default::default(),
                )
                .await
                .map_err(|e| e.to_string())?;
                info!("Module Loaded Successfully");
                let setup = match get_game_setup(&msg_db, config.oracle_key).await {
                    Ok(s) => s,
                    Err(e) => {
                        debug!(error=?e, "Error");
                        return Err(e)?;
                    }
                };
                info!(?setup, "Game Setup");
                // TODO: Game Amount from somewhere?
                let amt_per_player: AmountF64 =
                    AmountF64::from(Amount::from_sat(100000 / setup.players.len() as u64));
                let g = GameKernel {
                    game_host: PK(config.oracle_key),
                    players: setup
                        .players
                        .iter()
                        .map(|p| Ok((serde_json::from_str(&p)?, amt_per_player)))
                        .collect::<Result<_, serde_json::Error>>()?,
                    timeout: setup.finish_time,
                };
                // todo real args for contract
                let args = CreateArgs {
                    arguments: serde_json::to_value(&g).unwrap(),
                    context: ContextualArguments {
                        network: bitcoin::network::constants::Network::Bitcoin,
                        amount: Amount::from_sat(100000),
                        effects: Default::default(),
                    },
                };

                info!(?args, "Contract Args Ready");
                state.contract = if let Ok(c) = module.fresh_clone()?.call(&root, &args) {
                    info!(address=?c.address,"Contract Compilation Successful");
                    Ok(c)
                } else {
                    return Err("First Run of contract must pass")?;
                };
                state.args = Ok(args);
            }
            Some(Event::Rebind(o)) => {
                info!(output=?o, "Rebind");
                state.bound_to.insert(o);
            }
            Some(Event::NewRecompileTriggeringObservation(new_info_as_v)) => {
                info!("NewRecompileTriggeringObservation");
                let idx_str = SArc(Arc::new(format!("event-{}", state.event_counter)));
                // work on a clone so as to not have an effect if failed
                let mut new_args = state.args.as_ref().map_err(|e| e.as_str())?.clone();
                let mut save = EditableMapEffectDB::from(new_args.context.effects.clone());

                let mut any_edits = false;
                for api in state
                    .contract
                    .as_ref()
                    .map_err(|e| e.as_str())?
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
                    any_edits = true;
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

                if any_edits {
                    new_args.context.effects = save.into();
                    let result = {
                        let g_handle = state.module.lock().await;
                        // drop error before releasing g_handle so that the CompilationError non-send type
                        // doesn't get held across an await point
                        g_handle
                            .as_ref()
                            .map_err(|e| e.as_str())?
                            .fresh_clone()?
                            .call(&PathFragment::Root.into(), &new_args)
                            .map_err(|e| debug!(error=?e, "Module did not like the new argument"))
                    };

                    match result {
                        // TODO:  Belt 'n Suspsender Check:
                        // Check that old_state is augmented by new_state
                        Ok(new_contract) => {
                            let old_addr = state.contract.as_ref().map(|c| c.address.clone());
                            let new_addr = &new_contract.address;
                            if Script::from(old_addr.unwrap()) != Script::from(new_addr.clone()) {
                                Err("Critical Invariant Failed: Contract address mutated after recompile")?;
                            }

                            info!(address=?new_contract.address,"Contract ReCompilation Successful");
                            state.args = Ok(new_args);
                            state.contract = Ok(new_contract);
                        }
                        Err(e) => {}
                    }
                }
            }
            None => (),
        }

        // Post Event:

        state.event_counter += 1;
    })
}

const OK_T: Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> = Ok(());

fn read_move(
    e: Authenticated<GenericEnvelope<ParticipantAction>>,
) -> Result<(MoveEnvelope, XOnlyPublicKey), ()> {
    match e.msg() {
        ParticipantAction::MoveEnvelope(m) => Ok(((m.clone(), e.header().key()))),
        ParticipantAction::Custom(_) => Err(()),
        ParticipantAction::PsbtSigningCoordination(_) => Err(()),
    }
}
