use attest_database::connection::MsgDB;
use attest_database::setup_db;
use attest_util::bitcoin::BitcoinConfig;
use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::{OutPoint, Transaction, Txid};
use bitcoincore_rpc_async::Client;
use emulator_connect::{CTVAvailable, CTVEmulator};
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
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Notify;
mod simps;

#[derive(Serialize, Deserialize)]
enum Event {
    Initialization(CreateArgs<Value>),
    ExternalEvent(simps::Event),
    Rebind(OutPoint),
    SyntheticPeriodicActions,
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

/// Traverses a Compiled object and outputs all ContinuationPoints
// TODO: Do away with allocations?
struct ContractContinuations<'a>(
    Vec<&'a Compiled>,
    Option<Values<'a, SArc<EffectPath>, ContinuationPoint>>,
);

trait CompiledExt {
    fn continuation_points(&self) -> ContractContinuations;
}
impl CompiledExt for Compiled {
    fn continuation_points(&self) -> ContractContinuations {
        ContractContinuations(vec![], Some(self.continue_apis.values()))
    }
}
impl<'a> Iterator for ContractContinuations<'a> {
    type Item = &'a ContinuationPoint;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut value) = self.1 {
            let next = value.next();
            if next.is_some() {
                return next;
            } else {
                if let Some(contract) = self.0.pop() {
                    self.1 = Some(contract.continue_apis.values());
                    self.0.extend(
                        contract
                            .suggested_txs
                            .values()
                            .chain(contract.ctv_to_tx.values())
                            .flat_map(|t| t.outputs.iter())
                            .map(|out| &out.contract),
                    );
                }
                self.next()
            }
        } else {
            None
        }
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
    fn try_signing(&mut self, psbt: PartiallySignedTransaction) -> Option<Transaction> {
        // logic here should be to either try pinging services and attempt to finalize a txn out of it
        todo!()
    }
}

async fn get_db_from_env() -> Result<MsgDB, Box<dyn std::error::Error>> {
    let app_name = std::env::var("APP_DB_NAME")?;
    let prefix = std::env::var("APP_DB_PREFIX")
        .ok()
        .map(|m| PathBuf::from_str(&m))
        .transpose()?;
    let db = setup_db(&app_name, prefix).await?;
    Ok(db)
}

async fn get_bitcoin_rpc() -> Result<Arc<Client>, Box<dyn std::error::Error>> {
    let cfg: BitcoinConfig = serde_json::from_str(&std::env::var("APP_BTC_RPC_JSON")?)?;
    Ok(cfg.get_new_client().await?)
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _db = get_db_from_env().await?;
    let _rpc = get_bitcoin_rpc().await?;
    do_main().await
}

async fn do_main() -> Result<(), Box<dyn std::error::Error>> {
    let typ = "org";
    let org = "judica";
    let proj = "sapio-litigator";
    let proj =
        directories::ProjectDirs::from(typ, org, proj).expect("Failed to find config directory");
    let mut data_dir = proj.data_dir().to_owned();
    data_dir.push("modules");
    let emulator: Arc<dyn CTVEmulator> = Arc::new(CTVAvailable);
    let logfile = std::env::var("SAPIO_LITIGATOR_EVLOG")?;
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
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            loop {
                // TODO: Read lines after end?
                match lines.next_line().await {
                    Ok(Some(l)) => {
                        let e: Event = serde_json::from_str(&l)?;
                        tx.send(e).await;
                    }
                    Ok(None) => (),
                    _ => break,
                }
            }
            OK_T
        });
    }
    let periodic_action_complete = Arc::new(Notify::new());
    {
        let tx = tx.clone();
        let periodic_action_complete = periodic_action_complete.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                // order is important here: wait registers to get the signal before
                // tx.send enables the periodic call, guaranteeing it will see
                // the corresponding wake up
                //
                // we do this so that we don't ever have more than one active Periodic Action
                let wait = periodic_action_complete.notified();
                tx.send(Event::SyntheticPeriodicActions).await;
                wait.await;
            }
        });
    }
    let mut state = AppState::Uninitialized;
    loop {
        match rx.recv().await {
            Some(Event::SyntheticPeriodicActions) => {
                match &mut state {
                    AppState::Uninitialized => (),
                    AppState::Initialized {
                        args,
                        contract,
                        bound_to,
                        psbt_db,
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
                                            psbt,
                                            hex,
                                            metadata,
                                            output_metadata,
                                            added_output_metadata,
                                        } = tx;
                                        if let Some(data) =
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
                    args,
                    contract,
                    ref mut bound_to,
                    ..
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
                            return false;
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
