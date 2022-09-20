use attest_database::connection::MsgDB;
use attest_database::setup_db;
use attest_util::bitcoin::BitcoinConfig;
use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::{OutPoint, Transaction, Txid};
use bitcoincore_rpc_async::Client;
use emulator_connect::{CTVAvailable, CTVEmulator};
use event_log::connection::EventLog;
use event_log::db_handle::accessors::occurrence::{ApplicationTypeID, ToOccurrence};
use event_log::db_handle::accessors::occurrence_group::OccurrenceGroupKey;
use ruma_serde::CanonicalJsonValue;
use sapio::contract::abi::continuation::ContinuationPoint;
use sapio::contract::object::SapioStudioFormat;
use sapio::contract::{CompilationError, Compiled};
use sapio_base::effects::{EditableMapEffectDB, EffectPath, PathFragment};
use sapio_base::serialization_helpers::SArc;
use sapio_base::txindex::TxIndexLogger;
use sapio_wasm_plugin::host::PluginHandle;
use sapio_wasm_plugin::host::{plugin_handle::ModuleLocator, WasmPluginHandle};
use sapio_wasm_plugin::CreateArgs;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use simps::AutoBroadcast;
use std::collections::btree_map::Values;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Notify;

#[derive(Serialize, Deserialize)]
enum Event {
    Initialization(CreateArgs<Value>),
    ExternalEvent(simps::Event),
    Rebind(OutPoint),
    SyntheticPeriodicActions,
}

impl ToOccurrence for Event {
    fn to_data(&self) -> CanonicalJsonValue {
        ruma_serde::to_canonical_value(self).unwrap()
    }
    fn stable_typeid(&self) -> ApplicationTypeID {
        ApplicationTypeID::from_inner("LitigatorEvent")
    }
}

enum AppState {
    Uninitialized,
    Initialized {
        args: CreateArgs<Value>,
        contract: Compiled,
        bound_to: Option<OutPoint>,
        psbt_db: Arc<PSBTDatabase>,
    },
}
impl AppState {
    fn is_uninitialized(&self) -> bool {
        matches!(self, AppState::Uninitialized)
    }
}

trait CompiledExt {
    fn continuation_points<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContinuationPoint> + 'a>;
}
// TODO: Do away with allocations?
impl CompiledExt for Compiled {
    fn continuation_points<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContinuationPoint> + 'a> {
        Box::new(
            self.continue_apis.values().chain(
                self.suggested_txs
                    .values()
                    .chain(self.ctv_to_tx.values())
                    .flat_map(|x| &x.outputs)
                    .flat_map(|x| x.contract.continuation_points()),
            ),
        )
    }
}

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

#[derive(Deserialize)]
struct Config {
    db_app_name: String,
    #[serde(default)]
    db_prefix: Option<PathBuf>,
    bitcoin: BitcoinConfig,
    app_instance: String,
    logfile: PathBuf,
    event_log: EventLogConfig,
}

#[derive(Deserialize)]
struct EventLogConfig {
    app_name: String,
    #[serde(default)]
    prefix: Option<PathBuf>,
    group: OccurrenceGroupKey,
}

impl Config {
    fn from_env() -> Result<Config, Box<dyn std::error::Error>> {
        let j = std::env::var("LITIGATOR_CONFIG_JSON")?;
        Ok(serde_json::from_str(&j)?)
    }
    async fn get_db(&self) -> Result<MsgDB, Box<dyn std::error::Error>> {
        let db = setup_db(&self.db_app_name, self.db_prefix.clone()).await?;
        Ok(db)
    }
    async fn get_event_log(&self) -> Result<EventLog, Box<dyn std::error::Error>> {
        let db =
            event_log::setup_db(&self.event_log.app_name, self.event_log.prefix.clone()).await?;
        Ok(db)
    }
    async fn get_bitcoin_rpc(&self) -> Result<Arc<Client>, Box<dyn std::error::Error>> {
        Ok(self.bitcoin.get_new_client().await?)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    do_main().await
}

async fn do_main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(Config::from_env()?);
    let evlog = config.get_event_log().await?;
    let _db = config.get_db().await?;
    let _bitcoin = config.get_bitcoin_rpc().await?;
    let typ = "org";
    let org = "judica";
    let proj = format!("sapio-litigator.{}", config.app_instance);
    let proj =
        directories::ProjectDirs::from(typ, org, &proj).expect("Failed to find config directory");
    let mut data_dir = proj.data_dir().to_owned();
    data_dir.push("modules");
    let emulator: Arc<dyn CTVEmulator> = Arc::new(CTVAvailable);
    let logfile = config.logfile.clone();
    let mut opened = OpenOptions::default();
    opened.append(true).create(true).open(&logfile).await?;
    let fi = File::open(logfile).await?;
    let read = BufReader::new(fi);
    let mut lines = read.lines();
    let m: ModuleLocator = serde_json::from_str(
        &lines
            .next_line()
            .await?
            .expect("EVLog Should start with locator"),
    )?;
    let module = WasmPluginHandle::<Compiled>::new_async(
        &data_dir,
        &emulator,
        m,
        bitcoin::Network::Bitcoin,
        Default::default(),
    )
    .await?;
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let tx = tx.clone();
    let config = config.clone();
    let accessor = evlog.get_accessor().await;
    let key = &config.event_log.group;
    let evlog_group_id = accessor.get_occurrence_group_by_key(key);
    let evlog_group_id = evlog_group_id.or_else(|_| accessor.insert_new_occurrence_group(key))?;
    // This should be in notify_one mode, which means that only the db reader
    // should be calling notified and wakers should call notify_one.
    let new_synthetic_event = Arc::new(Notify::new());
    {
        let evlog = evlog.clone();
        let new_synthetic_event = new_synthetic_event.clone();
        tokio::spawn(async move {
            let mut last = None;
            loop {
                let wait_for_new_synth = {
                    let accessor = evlog.get_accessor().await;
                    let to_process = if let Some(last) = last {
                        accessor.get_occurrences_for_group_after_id(evlog_group_id, last)
                    } else {
                        accessor.get_occurrences_for_group(evlog_group_id)
                    }?;

                    for (occurrence_id, occurrence) in to_process {
                        let ev = Event::from_occurrence(occurrence)?;
                        if tx.send(ev).await.is_err() {
                            return Ok(());
                        }
                        last = Some(occurrence_id);
                    }
                    new_synthetic_event.notified()
                };
                tokio::select! {
                    _ = wait_for_new_synth => {}
                    _ = tokio::time::sleep(Duration::from_secs(30)) => {}
                }
            }
            OK_T
        });
    }
    let periodic_action_complete = Arc::new(Notify::new());
    {
        let periodic_action_complete = periodic_action_complete.clone();
        let evlog = evlog.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                // order is important here: wait registers to get the signal before
                // tx.send enables the periodic call, guaranteeing it will see
                // the corresponding wake up
                //
                // we do this so that we don't ever have more than one active Periodic Action
                let wait = periodic_action_complete.notified();
                {
                    let accessor = evlog.get_accessor().await;
                    let o: &dyn ToOccurrence = &Event::SyntheticPeriodicActions;
                    accessor.insert_new_occurrence_now_from(evlog_group_id, o)?;
                    new_synthetic_event.notify_one();
                }
                wait.await;
            }
            OK_T
        });
    }
    let mut state = AppState::Uninitialized;
    loop {
        match rx.recv().await {
            Some(Event::SyntheticPeriodicActions) => {
                match &mut state {
                    AppState::Uninitialized => (),
                    AppState::Initialized {
                        args: _,
                        contract,
                        bound_to,
                        psbt_db: _,
                    } => {
                        if let Some(out) = bound_to {
                            if let Ok(program) = contract.bind_psbt(
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
                }

                periodic_action_complete.notify_waiters()
            }
            Some(Event::Initialization(x)) => {
                if state.is_uninitialized() {
                    let init: Result<Compiled, CompilationError> =
                        module.call(&PathFragment::Root.into(), &x);
                    if let Ok(c) = init {
                        state = AppState::Initialized {
                            args: x,
                            contract: c,
                            bound_to: None,
                            psbt_db: Arc::new(PSBTDatabase::new()),
                        }
                    }
                }
            }
            Some(Event::Rebind(o)) => match &mut state {
                AppState::Uninitialized => todo!(),
                AppState::Initialized {
                    ref mut bound_to, ..
                } => {
                    bound_to.insert(o);
                }
            },
            Some(Event::ExternalEvent(e)) => match &state {
                AppState::Uninitialized => (),
                AppState::Initialized {
                    ref args,
                    ref contract,
                    ref bound_to,
                    psbt_db,
                } => {
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
                                    serde_json::from_value::<simps::EventRecompiler>(
                                        recompiler.clone(),
                                    )
                                {
                                    // Only pay attention to events that we are filtering for
                                    if recompiler.filter == e.key {
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
                                .map(|validator| validator.validate(&e.data).is_ok())
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
                            .insert(SArc(e.key.0.clone().into()), e.data.clone());
                    }

                    new_args.context.effects = save.into();
                    let new_state: Result<Compiled, CompilationError> =
                        module.call(&PathFragment::Root.into(), &new_args);
                    // TODO: Check that old_state is augmented by new_state
                    if let Ok(c) = new_state {
                        state = AppState::Initialized {
                            args: new_args,
                            contract: c,
                            bound_to: *bound_to,
                            psbt_db: psbt_db.clone(),
                        }
                    }
                    // TODO: Make it so that faulty effects are ignored.
                }
            },
            None => (),
        }
    }

    Ok(())
}

const OK_T: Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> = Ok(());
