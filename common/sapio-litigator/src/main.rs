use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::{OutPoint, Transaction, Txid};
use emulator_connect::{CTVAvailable, CTVEmulator};
use event_log::db_handle::accessors::occurrence_group::OccurrenceGroupID;
use futures::stream::FuturesUnordered;
use sapio::contract::Compiled;
use sapio_base::effects::PathFragment;
use sapio_wasm_plugin::host::WasmPluginHandle;
use sapio_wasm_plugin::CreateArgs;
use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::spawn;
use tokio::sync::{Mutex, Notify};
use tokio::task::JoinHandle;
use tracing::{info, trace};
mod universe;

pub use sapio_litigator_events as events;
pub mod litigator_event_log;

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
        Some(s) if &s == "run" => Ok(litigate_contract(config).await?),
        Some(s) => {
            println!("Invalid argument: {}", s);
            Ok(())
        }
    }
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
    let _ogid = evlog
        .get_accessor()
        .await
        .get_occurrence_group_by_key(&config.event_log.group)?;

    // check which continuation points need attest message routing

    let (tx, rx) = tokio::sync::mpsc::channel(1);
    let key = &config.event_log.group;
    let evlog_group_id = {
        let accessor = evlog.get_accessor().await;
        let evlog_group_id = accessor.get_occurrence_group_by_key(key);

        evlog_group_id.or_else(|_| accessor.insert_new_occurrence_group(key))?
    };
    // This should be in notify_one mode, which means that only the db reader
    // should be calling notified and wakers should call notify_one.
    let new_synthetic_event = Arc::new(Notify::new());
    let config = Arc::new(config);

    let tasks = FuturesUnordered::new();
    start_extractors(
        evlog.clone(),
        evlog_group_id,
        tx,
        new_synthetic_event,
        config.clone(),
        msg_db.clone(),
        &tasks,
    )
    .await
    .map_err(|e| e.to_string())?;

    let root = PathFragment::Root.into();
    let state = AppState {
        args: Err("No Args Loaded".into()),
        module: Arc::new(Mutex::new(Err("No Module Loaded".into()))),
        contract: Err("No Compiled Object".into()),
        bound_to: None,
        psbt_db: Arc::new(PSBTDatabase::new()),
        event_counter: 0,
    };
    let secp = Arc::new(Secp256k1::new());
    let evl = spawn(litigator_event_log::event_loop(
        rx,
        litigator_event_log::EventLoopContext {
            secp,
            state,
            emulator,
            data_dir,
            msg_db,
            config,
            root,
            evlog_group_id,
            evlog: evlog.clone(),
        },
    ));

    tasks.push(evl);

    for task in tasks {
        let r = task.await?;
        trace!(?r, "Error From Task Joining");
        r.map_err(|e| e.to_string())?;
    }
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
    evlog: event_log::connection::EventLog,
    evlog_group_id: OccurrenceGroupID,
    tx: tokio::sync::mpsc::Sender<events::Event>,
    new_synthetic_event: Arc<Notify>,
    config: Arc<config::Config>,
    msg_db: attest_database::connection::MsgDB,
    tasks: &TaskSet,
) -> Result<(), Box<dyn Error + Sync + Send + 'static>> {
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
        config.oracle_key,
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
