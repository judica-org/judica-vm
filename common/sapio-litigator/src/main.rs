use attest_database::connection::MsgDB;
use attest_database::setup_db;
use attest_util::bitcoin::BitcoinConfig;
use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::{OutPoint, Transaction, Txid, XOnlyPublicKey};
use bitcoincore_rpc_async::Client;
use emulator_connect::{CTVAvailable, CTVEmulator};
use event_log::connection::EventLog;
use event_log::db_handle::accessors::occurrence::{ApplicationTypeID, ToOccurrence};
use event_log::db_handle::accessors::occurrence_group::OccurrenceGroupKey;
use ruma_serde::CanonicalJsonValue;
use sapio::contract::abi::continuation::ContinuationPoint;
use sapio::contract::object::SapioStudioFormat;
use sapio::contract::{CompilationError, Compiled};
use sapio_base::effects::{EditableMapEffectDB, PathFragment};
use sapio_base::serialization_helpers::SArc;
use sapio_base::txindex::TxIndexLogger;
use sapio_wasm_plugin::host::PluginHandle;
use sapio_wasm_plugin::host::{plugin_handle::ModuleLocator, WasmPluginHandle};
use sapio_wasm_plugin::CreateArgs;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use simps::AutoBroadcast;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::spawn;
use tokio::sync::Notify;
mod universe;

#[derive(Serialize, Deserialize)]
pub enum Event {
    Initialization((Vec<u8>, CreateArgs<Value>)),
    ExternalEvent(simps::Event),
    TransactionFinalized(String, Transaction),
    Rebind(OutPoint),
    SyntheticPeriodicActions(i64),
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
    oracle_key: XOnlyPublicKey,
    contract_location: PathBuf,
    contract_args: Value,
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

fn data_dir_modules(app_instance: &str) -> PathBuf {
    let typ = "org";
    let org = "judica";
    let proj = format!("sapio-litigator.{}", app_instance);
    let proj =
        directories::ProjectDirs::from(typ, org, &proj).expect("Failed to find config directory");
    let mut data_dir = proj.data_dir().to_owned();
    data_dir.push("modules");
    data_dir
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let mut args = std::env::args();
    let config = Config::from_env()?;
    match args.nth(1) {
        None => todo!(),
        Some(s) if &s == "init" => Ok(init_contract_event_log(config).await?),
        Some(s) if &s == "run" => Ok(litigate_contract(config).await?),
        Some(s) => {
            println!("Invalid argument: {}", s);
            return Ok(());
        }
    }
}

async fn init_contract_event_log(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let evlog = config.get_event_log().await?;
    let accessor = evlog.get_accessor().await;
    let group = config.event_log.group;
    match accessor.get_occurrence_group_by_key(&group) {
        Ok(a) => {
            println!("Instance {} has already been initialized", group);
            return Ok(());
        }
        Err(e) => {
            let location = config.contract_location.to_str().unwrap();
            println!("Initializing contract at {}", location);
            println!("Using arguments {}", config.contract_args);
            let gid = accessor.insert_new_occurrence_group(&group)?;
            let ev = Event::Initialization((
                tokio::fs::read(&location).await?,
                serde_json::from_value(config.contract_args)?,
            ));
            let ev_occ: &dyn ToOccurrence = &ev;
            accessor.insert_occurrence(gid, &ev_occ.into())?;
            return Ok(());
        }
    }
}

// TODO: MAKE SURE WE ARE GRABBING CONTRACT PARAMETERS FROM BEGINNING OF EVENT LOG
async fn litigate_contract(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // initialize db connection to the event log
    let evlog = config.get_event_log().await?;
    let msg_db = config.get_db().await?;
    let _bitcoin = config.get_bitcoin_rpc().await?;

    // get location of project directory modules
    let data_dir = data_dir_modules(&config.app_instance);

    // allocate a CTV Emulator
    let emulator: Arc<dyn CTVEmulator> = Arc::new(CTVAvailable);

    // summon a wasm plugin handle
    let locator: ModuleLocator = ModuleLocator::FileName(format!(
        "{}",
        config
            .contract_location
            .into_os_string()
            .into_string()
            .expect("Couldn't convert OSStr to String")
    ));
    let module = WasmPluginHandle::<Compiled>::new_async(
        &data_dir,
        &emulator,
        locator,
        bitcoin::Network::Bitcoin,
        Default::default(),
    )
    .await?;
    let obj = module.call(
        &PathFragment::Root.into(),
        &serde_json::from_value(config.contract_args)?,
    )?;

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let accessor = evlog.get_accessor().await;
    let key = &config.event_log.group;
    let evlog_group_id = accessor.get_occurrence_group_by_key(key);
    let evlog_group_id = evlog_group_id.or_else(|_| accessor.insert_new_occurrence_group(key))?;
    // This should be in notify_one mode, which means that only the db reader
    // should be calling notified and wakers should call notify_one.
    let new_synthetic_event = Arc::new(Notify::new());
    let elp_evlog = evlog.clone();
    let elp_new_synthetic_event = new_synthetic_event.clone();
    tokio::spawn(async move {
        universe::linearized::event_log_processor(
            elp_evlog,
            evlog_group_id.clone(),
            tx,
            elp_new_synthetic_event,
        )
        .await?;
        OK_T
    });
    let tee_evlog = evlog.clone();
    let tee_new_synthetic_event = new_synthetic_event.clone();
    tokio::spawn(async move {
        universe::extractors::time::time_event_extractor(
            tee_evlog,
            evlog_group_id.clone(),
            tee_new_synthetic_event,
        )
        .await?;
        OK_T
    });
    let se_evlog = evlog.clone();
    let se_new_synthetic_event = new_synthetic_event.clone();
    tokio::spawn(async move {
        universe::extractors::sequencer::sequencer_extractor(
            config.oracle_key,
            msg_db,
            se_evlog,
            evlog_group_id,
            se_new_synthetic_event,
        )
    });

    let mut state = AppState::Uninitialized;
    loop {
        match rx.recv().await {
            Some(Event::TransactionFinalized(_s, _tx)) => {}
            Some(Event::SyntheticPeriodicActions(_t)) => {
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
            }
            Some(Event::Initialization(x)) => {
                if state.is_uninitialized() {
                    let init: Result<Compiled, CompilationError> =
                        module.call(&PathFragment::Root.into(), &x.1);
                    if let Ok(c) = init {
                        state = AppState::Initialized {
                            args: x.1,
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
