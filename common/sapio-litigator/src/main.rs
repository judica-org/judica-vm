use attest_database::connection::MsgDB;
use bitcoin::hashes::hex::ToHex;
use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::secp256k1::{All, Secp256k1};
use bitcoin::{OutPoint, Transaction, Txid, XOnlyPublicKey};
use bitcoincore_rpc_async::Client;
use emulator_connect::{CTVAvailable, CTVEmulator};
use event_log::connection::EventLog;
use event_log::db_handle::accessors::occurrence_group::OccurrenceGroupID;
use futures::stream::FuturesUnordered;
use sapio::contract::Compiled;
use sapio_base::effects::{EffectPath, PathFragment};
pub use sapio_litigator_events as events;
use sapio_wasm_plugin::host::WasmPluginHandle;
use sapio_wasm_plugin::CreateArgs;
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::spawn;
use tokio::sync::{Mutex, Notify};
use tokio::task::JoinHandle;
use tracing::{debug, trace};
pub mod litigator_event_log;
mod universe;

struct GlobalLitigatorState {
    config: Arc<config::Config>,
    emulator: Arc<dyn CTVEmulator>,
    evlog: EventLog,
    msg_db: MsgDB,
    bitcoin: Arc<Client>,
    data_dir: PathBuf,
    secp: Arc<Secp256k1<All>>,
    running_instances: Mutex<BTreeMap<XOnlyPublicKey, InstanceRuntime>>,
    scan_for_new_contracts: Duration,
}

struct LitigatedContractInstanceState {
    bound_to: Option<OutPoint>,
    psbt_db: Arc<PSBTDatabase>,
    // Initialized after first move
    module: Arc<Mutex<Result<WasmPluginHandle<Compiled>, String>>>,
    args: Result<CreateArgs<Value>, String>,
    contract: Result<Compiled, String>,
    event_counter: u64,
    root: EffectPath,
    new_synthetic_event: Arc<Notify>,
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

struct InstanceRuntime {
    task: Option<JoinHandle<TaskType>>,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let mut args = std::env::args();
    let config = Arc::new(config::Config::from_env()?);
    let globals = Arc::new(GlobalLitigatorState {
        // initialize db connection to the event log
        evlog: config.get_event_log().await?,
        // get location of project directory modules
        msg_db: config.get_db().await?,
        // allocate a CTV Emulator
        bitcoin: config.get_bitcoin_rpc().await?,
        data_dir: config::data_dir_modules(&config.app_instance),
        config,
        emulator: Arc::new(CTVAvailable),
        secp: Arc::new(Secp256k1::new()),
        running_instances: Mutex::new(BTreeMap::<XOnlyPublicKey, InstanceRuntime>::new()),
        scan_for_new_contracts: Duration::from_secs(10),
    });
    match args.nth(1) {
        None => Err("Must have init or run")?,
        Some(s) if &s == "run" => loop {
            trace!(instance = "Global", "Scanning for Tasks");
            // First shut down all crashed events...
            {
                let mut instances = globals.running_instances.lock().await;
                for (key, instance) in instances.iter_mut() {
                    trace!(instance = key.to_hex(), "Scanning for Tasks");
                    if Some(true) == instance.task.as_ref().map(|t| t.is_finished()) {
                        let finished = instance.task.take().expect("CHecked on if condition").await;
                        debug!(result=?finished, instance=?key, "Quit Task");
                    } else {
                        trace!(instance = key.to_hex(), "Healthy");
                    }
                }

                // TODO: Remove crashed tasks/ log so they can restart?
            }
            {
                let accessor = globals.evlog.get_accessor().await;
                let groups = accessor.get_all_occurrence_groups()?;
                let mut instances = globals.running_instances.lock().await;
                let new_instances: Vec<_> = groups
                    .iter()
                    .filter_map(|(_id, k)| XOnlyPublicKey::from_str(k).ok())
                    // TODO: Remove crashed tasks/ log so they can restart? maybe also do if is none task?
                    .filter(|k| !instances.contains_key(k))
                    .collect();
                for new_instance in new_instances {
                    trace!(instance = new_instance.to_hex(), "New Instance");
                    let cglobals = globals.clone();
                    instances.insert(
                        new_instance,
                        InstanceRuntime {
                            task: Some(spawn(litigate_contract(cglobals, new_instance))),
                        },
                    );
                }
            }

            tokio::time::sleep(globals.scan_for_new_contracts).await;
        },
        Some(s) => {
            println!("Invalid argument: {}", s);
            Ok(())
        }
    }
}

type TaskType = Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;
async fn litigate_contract(globals: Arc<GlobalLitigatorState>, key: XOnlyPublicKey) -> TaskType {
    // check which continuation points need attest message routing
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    let group_key = key.to_hex();
    let evlog_group_id = {
        let accessor = globals.evlog.get_accessor().await;
        accessor
            .get_occurrence_group_by_key(&group_key)
            .expect("Must already exist if being litigated")
    };
    // This should be in notify_one mode, which means that only the db reader
    // should be calling notified and wakers should call notify_one.
    let tasks = FuturesUnordered::new();

    let state = LitigatedContractInstanceState {
        args: Err("No Args Loaded".into()),
        module: Arc::new(Mutex::new(Err("No Module Loaded".into()))),
        contract: Err("No Compiled Object".into()),
        bound_to: None,
        psbt_db: Arc::new(PSBTDatabase::new()),
        event_counter: 0,
        root: PathFragment::Root.into(),
        new_synthetic_event: Arc::new(Notify::new()),
    };

    start_extractors(
        key,
        globals.clone(),
        state.new_synthetic_event.clone(),
        evlog_group_id,
        tx,
        &tasks,
    )
    .await
    .map_err(|e| e.to_string())?;

    let evl = spawn(litigator_event_log::event_loop(
        rx,
        litigator_event_log::EventLoopContext {
            state,
            globals: globals.clone(),
            evlog_group_id,
        },
    ));

    tasks.push(evl);

    for task in tasks {
        let r = task.await?;
        trace!(?r, "Error From Task Joining");
        r.map_err(|e| e.to_string())?;
    }
    // This can only be reached if all sub-tasks did not return an error
    Ok(())
}

pub type TaskSet = FuturesUnordered<
    JoinHandle<
        std::result::Result<
            (),
            std::boxed::Box<
                (dyn std::error::Error + std::marker::Send + std::marker::Sync + 'static),
            >,
        >,
    >,
>;
async fn start_extractors(
    sequencer_key: XOnlyPublicKey,
    globals: Arc<GlobalLitigatorState>,
    new_synthetic_event: Arc<Notify>,
    evlog_group_id: OccurrenceGroupID,
    tx: tokio::sync::mpsc::Sender<events::Event>,
    tasks: &TaskSet,
) -> Result<(), Box<dyn Error + Sync + Send + 'static>> {
    let evlog: event_log::connection::EventLog = globals.evlog.clone();
    let msg_db: attest_database::connection::MsgDB = globals.msg_db.clone();
    let _config: Arc<config::Config> = globals.config.clone();
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
    universe::extractors::sequencer::sequencer_extractor(
        sequencer_key,
        msg_db.clone(),
        evlog.clone(),
        evlog_group_id,
        new_synthetic_event.clone(),
        tasks,
    )
    .await?;

    tasks.push(tokio::spawn(universe::extractors::dlog::dlog_extractor(
        msg_db.clone(),
        evlog.clone(),
        evlog_group_id,
        Duration::from_secs(10),
    )));
    Ok(())
}

const OK_T: Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> = Ok(());
