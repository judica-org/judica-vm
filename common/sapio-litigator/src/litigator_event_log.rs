use super::AppState;
use super::OK_T;
use crate::{config, events, ext::CompiledExt, universe::extractors::sequencer::get_game_setup};
use attest_database::db_handle::create::TipControl;
use attest_messages::Authenticated;
use attest_messages::GenericEnvelope;
use bitcoin::{
    blockdata::script::Script,
    hashes::{sha256, sha512, Hash, Hmac, HmacEngine},
    psbt::PartiallySignedTransaction,
    secp256k1::{All, Secp256k1},
    util::bip32::{ChainCode, ChildNumber, ExtendedPrivKey, Fingerprint},
    Amount, KeyPair, OutPoint, XOnlyPublicKey,
};
use emulator_connect::CTVEmulator;
use event_log::{
    connection::EventLog,
    db_handle::accessors::{occurrence::sql::Idempotent, occurrence_group::OccurrenceGroupID},
};
use game_player_messages::{Multiplexed, ParticipantAction, PsbtString};
use mine_with_friends_board::MoveEnvelope;
use sapio::contract::object::SapioStudioFormat;
use sapio::contract::Compiled;
use sapio::util::amountrange::AmountF64;
use sapio_base::{
    effects::{EditableMapEffectDB, PathFragment},
    serialization_helpers::SArc,
    simp::{by_simp, SIMP},
    txindex::TxIndexLogger,
};
use sapio_psbt::SigningKey;
use sapio_wasm_plugin::{
    host::{plugin_handle::ModuleLocator, WasmPluginHandle},
    plugin_handle::PluginHandle,
    ContextualArguments, CreateArgs,
};
use serde_json::Value;
use simps::{self, EventKey, GameKernel, GameStarted, PK};
use std::collections::BTreeMap;
use std::error::Error;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tracing::debug;
use tracing::info;

pub(crate) struct EventLoopContext {
    pub(crate) state: AppState,
    pub(crate) emulator: Arc<dyn CTVEmulator>,
    pub(crate) data_dir: std::path::PathBuf,
    pub(crate) msg_db: attest_database::connection::MsgDB,
    pub(crate) config: Arc<config::Config>,
    pub(crate) root: sapio_base::reverse_path::ReversePath<PathFragment>,
    pub(crate) evlog_group_id: OccurrenceGroupID,
    pub(crate) evlog: EventLog,
    pub(crate) secp: Arc<Secp256k1<All>>,
}

pub(crate) async fn event_loop(
    mut rx: Receiver<events::Event>,
    mut e: EventLoopContext,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    loop {
        match rx.recv().await {
            Some(events::Event::EmittedPSBTVia(_a, _b)) => {}
            Some(events::Event::TransactionFinalized(_s, _tx)) => {
                info!("Transaction Finalized");
            }
            Some(events::Event::SyntheticPeriodicActions(time)) => {
                handle_synthetic_periodic(&mut e, time).await?;
            }
            Some(events::Event::ModuleBytes(contract_bytes)) => {
                handle_module_bytes(&mut e, contract_bytes).await?;
            }
            Some(events::Event::Rebind(o)) => {
                handle_rebind(&mut e, o);
            }
            Some(events::Event::NewRecompileTriggeringObservation(new_info_as_v, filter)) => {
                handle_new_information(&mut e, filter, new_info_as_v).await?;
            }
            None => (),
        }

        // Post Event:

        e.state.event_counter += 1;
    }
    OK_T
}

pub(crate) async fn handle_new_information(
    e: &mut EventLoopContext,
    filter: SArc<EventKey>,
    new_info_as_v: Value,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let EventLoopContext { ref mut state, .. } = e;
    info!("NewRecompileTriggeringObservation");
    let idx_str = SArc(Arc::new(format!("event-{}", state.event_counter)));
    let mut new_args = state.args.as_ref().map_err(|e| e.as_str())?.clone();
    let mut save = EditableMapEffectDB::from(new_args.context.effects.clone());
    let mut any_edits = false;
    for api in state
        .contract
        .as_ref()
        .map_err(|e| e.as_str())?
        .continuation_points()
        .filter(|api| {
            (&api.simp >> by_simp::<simps::EventRecompiler>())
                .and_then(|j| simps::EventRecompiler::from_json(j.clone()).ok())
                .map(|j| j.filter == *filter.0)
                .unwrap_or(false)
        })
        .filter(|api| {
            api.schema
                .as_ref()
                .and_then(|schema| {
                    // todo: cache?
                    jsonschema_valid::Config::from_schema(
                        // since schema is a RootSchema, cannot throw here
                        &schema.0,
                        Some(jsonschema_valid::schemas::Draft::Draft6),
                    )
                    .ok()
                })
                .map(|validator| validator.validate(&new_info_as_v).is_ok())
                .unwrap_or(false)
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
            Err(_e) => {}
        }
    };
    Ok(())
}

pub(crate) fn handle_rebind(e: &mut EventLoopContext, o: OutPoint) {
    let EventLoopContext { ref mut state, .. } = e;
    info!(output=?o, "Rebind");
    state.bound_to.insert(o);
}

pub(crate) async fn handle_module_bytes(
    e: &mut EventLoopContext,
    contract_bytes: Vec<u8>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let EventLoopContext {
        ref mut state,
        ref emulator,
        ref data_dir,
        ref msg_db,
        ref config,
        ref root,
        ..
    } = e;
    info!("ModuleBytes");
    let locator: ModuleLocator = ModuleLocator::Bytes(contract_bytes);
    let module = WasmPluginHandle::<Compiled>::new_async(
        &data_dir,
        emulator,
        locator,
        bitcoin::Network::Bitcoin,
        Default::default(),
    )
    .await
    .map_err(|e| e.to_string())?;
    info!("Module Loaded Successfully");
    let setup = match get_game_setup(msg_db, config.oracle_key).await {
        Ok(s) => s,
        Err(e) => {
            debug!(error=?e, "Error");
            return Err(e)?;
        }
    };
    info!(?setup, "Game Setup");
    let amt_per_player: AmountF64 =
        AmountF64::from(Amount::from_sat(100000 / setup.players.len() as u64));
    let g = GameKernel {
        game_host: PK(config.oracle_key),
        players: setup
            .players
            .iter()
            .map(|p| Ok((PK(XOnlyPublicKey::from_str(p)?), amt_per_player)))
            .collect::<Result<_, bitcoin::secp256k1::Error>>()
            .map_err(|e| {
                format!(
                    "Failed To Make JSON {}:{}\n    Error: {:?}",
                    file!(),
                    line!(),
                    e
                )
            })?,
        timeout: setup.finish_time,
    };
    info!(?g, "Game Kernel");
    let args = CreateArgs {
        arguments: serde_json::to_value(&GameStarted { kernel: g }).unwrap(),
        context: ContextualArguments {
            network: bitcoin::network::constants::Network::Bitcoin,
            amount: Amount::from_sat(100000),
            effects: Default::default(),
        },
    };
    info!(?args, "Contract Args Ready");
    state.contract = match module.call(root, &args) {
        Ok(c) => {
            info!(address=?c.address,"Contract Compilation Successful");
            Ok(c)
        }
        Err(e) => {
            debug!(error=?e, "Contract Failed to Compiled");
            return Err("First Run of contract must pass")?;
        }
    };
    state.args = Ok(args);
    Ok(())
}

pub(crate) async fn handle_synthetic_periodic(
    e: &mut EventLoopContext,
    time: i64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let EventLoopContext {
        ref mut state,
        ref emulator,
        ref secp,
        ref msg_db,
        ref config,
        ref evlog_group_id,
        ref evlog,
        ..
    } = e;
    info!(time, "SyntehticPeriodicActions");
    if let Some(out) = state.bound_to.as_ref() {
        let c = &state.contract.as_ref().map_err(|e| e.as_str())?;
        if let Ok(program) = bind_psbt(c, out, emulator) {
            // TODO learn available keys through an extractor...
            let keys = Arc::new({
                let handle = msg_db.get_handle().await;
                handle.get_keymap().map_err(|e| e.to_string())
            }?);
            for obj in program.program.into_values() {
                for tx in obj.txs.into_iter() {
                    let SapioStudioFormat::LinkedPSBT {
                        psbt,
                        hex: _,
                        metadata,
                        output_metadata: _,
                        added_output_metadata: _,
                    } = tx;
                    let keys = keys.clone();
                    let secp = secp.clone();
                    let config = config.clone();
                    let msg_db = msg_db.clone();
                    // put this in an async block to simplify error handling
                    let r = process_psbt_fail_ok(
                        keys,
                        config,
                        psbt,
                        metadata,
                        secp,
                        msg_db,
                        *evlog_group_id,
                        evlog.clone(),
                    )
                    .await;
                    if let Err(r) = r {
                        debug!(error=?r, "Failed PSBT Signing for this PSBT")
                    }
                }
            }
        }
    };
    Ok(())
}

pub(crate) async fn process_psbt_fail_ok(
    keys: Arc<BTreeMap<XOnlyPublicKey, bitcoin::secp256k1::SecretKey>>,
    config: Arc<config::Config>,
    psbt: String,
    metadata: sapio::template::TemplateMetadata,
    secp: Arc<Secp256k1<bitcoin::secp256k1::All>>,
    msg_db: attest_database::connection::MsgDB,
    evlog_group_id: OccurrenceGroupID,
    evlog: EventLog,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let broadcast_key = keys
        .get(&config.psbt_broadcast_key)
        .ok_or("Broadcast Key Unknown")?;
    let psbt = PartiallySignedTransaction::from_str(&psbt)?;
    let skeys = extract_keys_for_simp(metadata, keys.clone())?;
    let signing_key = SigningKey(skeys);
    let signed = signing_key
        .sign_psbt(
            psbt.clone(),
            &secp,
            bitcoin::SchnorrSighashType::AllPlusAnyoneCanPay,
        )
        .map_err(|(_old, e)| e)?;
    if signed == psbt {
        return OK_T;
    }
    let keypair = KeyPair::from_secret_key(&secp, broadcast_key);

    let tx = signed.clone().extract_tx();
    let txid = tx.txid();
    let txid_s = txid.to_string();

    let emitter = keypair.x_only_public_key().0;
    // TODO: confirm serialization is deterministic?
    let psbt_hash = sha256::Hash::hash(signed.to_string().as_bytes());
    let o = events::TaggedEvent(
        events::Event::EmittedPSBTVia(PsbtString(signed.clone()), emitter),
        Some(events::Tag::ScopedValue(
            "signed_psbt".into(),
            format!("emit_by:{}:psbt_hash:{}", emitter, psbt_hash),
        )),
    );
    let mut accessor = evlog.get_accessor().await;
    let mut handle = msg_db.get_handle().await;
    let v = accessor.insert_new_occurrence_now_from_txn(evlog_group_id, &o);
    let v2 = v?;
    match v2 {
        Err(Idempotent::AlreadyExists) => Ok(()),
        Ok((_oid, txn)) => {
            handle.retry_insert_authenticated_envelope_atomic::<ParticipantAction, _, _>(
                ParticipantAction::PsbtSigningCoordination(Multiplexed {
                    channel: txid_s,
                    data: PsbtString(signed),
                }),
                &keypair,
                &secp,
                None,
                TipControl::NoTips,
            )?;
            // Technically there is a tiny risk that we succeed at inserting the
            // Signed PSBT but do not succeed at committing the event log entry.
            // In this case, we will see a second entry for the same psbt, which
            // is still not a logic error, fortunately.
            //
            // This could be fixed with some more clever logic in both DBs.
            txn.commit()?;
            Ok(())
        }
    }
}

pub(crate) fn extract_keys_for_simp(
    metadata: sapio::template::TemplateMetadata,
    keys: Arc<BTreeMap<XOnlyPublicKey, bitcoin::secp256k1::SecretKey>>,
) -> Result<Vec<ExtendedPrivKey>, Box<dyn Error + Send + Sync>> {
    let auto = (&metadata.simp >> by_simp::<simps::AutoBroadcast>()).ok_or("No AutoBroadcast")?;
    let auto = simps::AutoBroadcast::from_json(auto.clone())?;
    let mut skeys = vec![];
    for private_key in auto
        .signer_roles
        .iter()
        .filter(|(_, o)| o.sign && o.sign_all)
        .filter_map(|(PK(pk), _)| keys.get(pk).cloned())
    {
        let hmac_engine: HmacEngine<sha512::Hash> = HmacEngine::new(&private_key[..]);
        let hmac_result: Hmac<sha512::Hash> = Hmac::from_engine(hmac_engine);
        skeys.push(ExtendedPrivKey {
            // todo: other networks
            network: bitcoin::Network::Signet,
            depth: 0,
            parent_fingerprint: Fingerprint::default(),
            child_number: ChildNumber::from(0),
            private_key,
            // Garbage Chaincode, but secure to work in theory.
            // TODO: store EPKs in DB?
            chain_code: ChainCode::from(&hmac_result[32..]),
        });
    }
    Ok(skeys)
}

pub(crate) fn bind_psbt(
    c: &&Compiled,
    out: &OutPoint,
    emulator: &Arc<dyn CTVEmulator>,
) -> Result<sapio::contract::object::Program, Box<dyn Error + Send + Sync + 'static>> {
    Ok(c.bind_psbt(
        *out,
        Default::default(),
        Rc::new(TxIndexLogger::new()),
        emulator.as_ref(),
    )
    .map_err(|e| e.to_string())?)
}

pub(crate) fn read_move(
    e: Authenticated<GenericEnvelope<ParticipantAction>>,
) -> Result<(MoveEnvelope, XOnlyPublicKey), ()> {
    match e.msg() {
        ParticipantAction::MoveEnvelope(m) => Ok((m.clone(), e.header().key())),
        ParticipantAction::Custom(_) => Err(()),
        ParticipantAction::PsbtSigningCoordination(_) => Err(()),
    }
}
