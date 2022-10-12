// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use app::CompilerModule;
use attest_database::setup_db;
use attest_database::{connection::MsgDB, db_handle::create::TipControl};
use attest_messages::{
    Authenticated, CanonicalEnvelopeHash, Envelope, GenericEnvelope, WrappedJson,
};
use attest_util::bitcoin::BitcoinConfig;
use emulator_connect::{CTVAvailable, CTVEmulator};

use event_log::db_handle::accessors::occurrence::ToOccurrence;
use game_host_messages::{BroadcastByHost, Channelized};
use sapio::contract::Compiled;
use sapio_bitcoin::secp256k1::rand;
use sapio_bitcoin::secp256k1::rand::seq::SliceRandom;
use sapio_bitcoin::secp256k1::All;
use sapio_bitcoin::Network;
use sapio_bitcoin::{secp256k1::Secp256k1, KeyPair};
use sapio_litigator_events::ModuleRepo;
use sapio_wasm_plugin::host::plugin_handle::ModuleLocator;
use sapio_wasm_plugin::host::WasmPluginHandle;

use serde::{Deserialize, Serialize};

use std::time::Duration;
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    error::Error,
    path::PathBuf,
    sync::Arc,
};
use tokio::spawn;
use tokio::sync::Mutex;
use tokio::task::{spawn_blocking, JoinHandle};
use tor::TorConfig;
use tracing::{debug, info, warn};

use crate::globals::GlobalsInner;
mod app;
mod globals;
mod tor;
#[derive(Serialize, Deserialize)]
pub struct Config {
    tor: TorConfig,
    #[serde(default)]
    prefix: Option<PathBuf>,
    game_host_name: String,
    pub(crate) bitcoin: BitcoinConfig,
    pub(crate) contract_location: String,
    app_instance: String,
    event_log: EventLogConfig,
    bitcoin_network: Network,
}
#[derive(Serialize, Deserialize)]
pub struct EventLogConfig {
    app_name: String,
    #[serde(default)]
    prefix: Option<PathBuf>,
}

fn get_config() -> Result<Arc<Config>, Box<dyn Error + Send + Sync>> {
    let config = std::env::var("GAME_HOST_CONFIG_JSON").map(|s| serde_json::from_str(&s))??;
    Ok(Arc::new(config))
}

pub(crate) fn data_dir_modules(app_instance: &str) -> PathBuf {
    let typ = "org";
    let org = "judica";
    let proj = format!("sapio-game-host.{}", app_instance);
    let proj =
        directories::ProjectDirs::from(typ, org, &proj).expect("Failed to find config directory");
    let mut data_dir = proj.data_dir().to_owned();
    data_dir.push("modules");
    data_dir
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let config = get_config()?;

    let client = config.bitcoin.get_new_client().await?;
    // get location of project directory modules
    let data_dir = data_dir_modules(&config.app_instance);

    // Connect to litigator's DB
    let proj = format!("sapio-litigator.{}", config.event_log.app_name);
    let evlog = event_log::setup_db(&proj, config.event_log.prefix.clone())
        .await
        .map_err(|e| e.to_string())?;
    let (module_bytes, module_tag, module_repo_id) = {
        let module_bytes = tokio::fs::read(&config.contract_location).await?;
        let accessor = evlog.get_accessor().await;
        let mrk = ModuleRepo::default_group_key();
        let gid = accessor
            .get_occurrence_group_by_key(&mrk)
            .or_else(|_| accessor.insert_new_occurrence_group(&mrk))
            .or_else(|_| accessor.get_occurrence_group_by_key(&mrk))?;
        let mr = ModuleRepo(module_bytes);
        let tag = mr.unique_tag().unwrap();
        // get or insert or get
        let _a = accessor
            .get_occurrence_for_group_by_tag(gid, &tag)
            .map(|(i, _)| i)
            .or_else(|_| {
                accessor
                    .insert_new_occurrence_now_from(gid, &mr)?
                    .or_else(|_Idempotent| {
                        accessor
                            .get_occurrence_for_group_by_tag(gid, &tag)
                            .map(|(i, _)| i)
                    })
            })?;
        (mr.0, tag, gid)
    };
    let locator: ModuleLocator = ModuleLocator::Bytes(module_bytes);

    let emulator: Arc<dyn CTVEmulator> = Arc::new(CTVAvailable);
    let compiler_module: CompilerModule = Arc::new(Mutex::new(
        WasmPluginHandle::<Compiled>::new_async(
            &data_dir,
            &emulator,
            locator,
            config.bitcoin_network,
            Default::default(),
        )
        .await
        .map_err(|e| e.to_string())?,
    ));
    let globals = Arc::new(GlobalsInner {
        module_repo_id,
        module_tag,
        evlog,
        compiler_module,
        bitcoin_rpc: client,
        bitcoin_network: config.bitcoin_network,
    });

    let db = setup_db(
        &format!("attestations.{}", config.game_host_name),
        config.prefix.clone(),
    )
    .await
    .map_err(|e| format!("DB Setup Failed: {:?}", e))?;
    let tor_server = tor::start(config.clone()).await;

    let host = config.tor.get_hostname().await?;
    info!("Hosting Onion Service At: {}", host);

    let app_instance = app::run(config.clone(), db.clone(), globals);
    let game_instance = game_server(config, db.clone());
    tokio::select! {
        a =  game_instance =>{
            a?;
        },
        b = app_instance => {
            b?.map_err(|e| format!("{}", e))?;
        }
        tor_quit = tor_server => {
            tor_quit?.map_err(|e| format!("{}", e))?;
        }
    }
    Ok(())
}

async fn game_server(config: Arc<Config>, db: MsgDB) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut task_set = BTreeMap::<_, JoinHandle<_>>::new();
    let secp = Arc::new(Secp256k1::new());
    loop {
        info!("Task Creator Starting Jobs for each key");
        let keymap = {
            let handle = db.get_handle_read().await;
            spawn_blocking(move || handle.get_keymap()).await??
        };
        for (key, value) in keymap {
            match task_set.entry(key) {
                std::collections::btree_map::Entry::Vacant(e) => {
                    info!(?key, "No Task Found, starting new game task...");
                    let keypair = KeyPair::from_secret_key(&secp, &value);
                    e.insert(spawn(game(
                        config.clone(),
                        db.clone(),
                        keypair,
                        secp.clone(),
                    )));
                }
                std::collections::btree_map::Entry::Occupied(ref mut x) => {
                    if x.get().is_finished() {
                        info!(?key, "Task Quit, rebooting...");
                        let keypair = KeyPair::from_secret_key(&secp, &value);
                        let old = x.insert(spawn(game(
                            config.clone(),
                            db.clone(),
                            keypair,
                            secp.clone(),
                        )));
                        let res = old.await;
                        debug!(?res, ?key, "Game Task Quit with");
                    } else {
                        info!(?key, "Task Healthy");
                    }
                }
            }
        }
        info!("Task Creator Sleeping");
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

async fn game(
    _config: Arc<Config>,
    db: MsgDB,
    keypair: KeyPair,
    secp: Arc<Secp256k1<All>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let oracle_publickey = keypair.public_key().x_only_public_key().0;
    let mut already_sequenced: Vec<CanonicalEnvelopeHash> = vec![];
    // First we get all of the old messages for the Oracle itself, so that we
    // can know which messages we've sequenced previously.
    {
        let v: Vec<Authenticated<GenericEnvelope<Channelized<BroadcastByHost>>>> = {
            let handle = db.get_handle_read().await;
            spawn_blocking(move || {
                handle.load_all_messages_for_user_by_key_connected(&oracle_publickey)
            })
            .await??
        };
        let sequencer = game_sequencer::RawSequencer {
            sequencer_envelopes: v,
            msg_cache: Default::default(),
        };
        let sequencer: game_sequencer::OfflineSequencer<game_player_messages::ParticipantAction> =
            sequencer.try_into()?;
        for batch in sequencer.batches_to_sequence {
            already_sequenced.extend(batch.iter());
        }
    }
    info!(
        n = already_sequenced.len(),
        "Loaded Previous Sequence Messages"
    );
    let mut all_unprocessed_messages = HashMap::new();
    let mut message_by_genesis = HashMap::<CanonicalEnvelopeHash, BTreeMap<i64, Envelope>>::new();
    let mut next_height_to_sequence: HashMap<CanonicalEnvelopeHash, i64> = Default::default();
    let mut seq = 0;

    // Get All the messages that we've not yet seen, but incosistently
    // Incosistency means that we may still be fetching priors tips in our
    // network stack.
    //
    // Only fetch the messages for our groups though
    {
        let handle = db.get_handle_read().await;
        let mut all_unprocessed_messages_tmp_in = Default::default();
        std::mem::swap(
            &mut all_unprocessed_messages,
            &mut all_unprocessed_messages_tmp_in,
        );
        let mut seq_tmp_in = seq;
        let (mut all_unprocessed_messages_tmp_out, seq_tmp_out) =  spawn_blocking(move ||
            {handle
                .get_all_chain_commit_group_members_new_envelopes_for_chain_into_inconsistent::<Authenticated<Envelope>, WrappedJson>(
                    oracle_publickey,
                    &mut seq_tmp_in,
                    &mut all_unprocessed_messages_tmp_in,
                ).and( Ok((all_unprocessed_messages_tmp_in, seq_tmp_in)))

            }
            ).await??;
        seq = seq_tmp_out;
        std::mem::swap(
            &mut all_unprocessed_messages,
            &mut all_unprocessed_messages_tmp_out,
        );
    }
    // Only on the first pass, remove the messages that have already been sequenced
    //
    // Later passes will be cleared.
    //
    // Also record the prior max sequenced.
    for m in already_sequenced.into_iter() {
        if let Some(msg) = all_unprocessed_messages.remove(&m) {
            let r = next_height_to_sequence
                .entry(msg.get_genesis_hash())
                .or_default();
            *r = std::cmp::max(msg.header().height() + 1, *r);
        } else {
            debug!(message_hash=?m, "Missing Message!");
            panic!("Message was sequenced, but was not contained in database");
        }
    }
    info!(
        n = next_height_to_sequence.len(),
        "Found Existing Comitted Chains"
    );
    loop {
        // Get All the messages that we've not yet seen, but incosistently
        // Incosistency means that we may still be fetching priors tips in our
        // network stack.
        //
        // Only fetch the messages for our groups though
        {
            let handle = db.get_handle_read().await;
            let mut all_unprocessed_messages_tmp_in = Default::default();
            std::mem::swap(
                &mut all_unprocessed_messages,
                &mut all_unprocessed_messages_tmp_in,
            );
            let mut seq_tmp_in = seq;
            let (mut all_unprocessed_messages_tmp_out, seq_tmp_out) =  spawn_blocking(move ||
                {
                    handle
                    .get_all_chain_commit_group_members_new_envelopes_for_chain_into_inconsistent::<Authenticated<Envelope>, WrappedJson>(
                        oracle_publickey,
                        &mut seq_tmp_in,
                        &mut all_unprocessed_messages_tmp_in,
                    ).and(Ok((all_unprocessed_messages_tmp_in, seq_tmp_in)))
                }
                ).await??;
            seq = seq_tmp_out;
            std::mem::swap(
                &mut all_unprocessed_messages,
                &mut all_unprocessed_messages_tmp_out,
            );
        }

        info!(
            n = all_unprocessed_messages.len(),
            "Got New Messages to Sequence"
        );
        //  Filter out events by the oracle and sort events by particular user
        {
            let unprocessed_message_keys =
                all_unprocessed_messages.keys().cloned().collect::<Vec<_>>();
            for value in &unprocessed_message_keys {
                // we can remove it now because the only reason we will drop it is if it is not to be sequenced
                if let Some((_k, e)) = all_unprocessed_messages.remove_entry(value) {
                    if e.header().key() == oracle_publickey {
                        panic!("Chain Cannot Listen to itself, logic error");
                    }
                    if message_by_genesis
                        .entry(e.get_genesis_hash())
                        .or_default()
                        .insert(e.header().height(), e.inner())
                        .is_some()
                    {
                        warn!("Should Never Be Possible to Have a Duplicate Message Here");
                        // TODO: Panic Always?
                        panic!("Test Invariant Failed");
                    }
                }
            }
        }
        // Sort the new entries
        let mut to_sequence = VecDeque::new();
        for (genesis, ms) in message_by_genesis.iter_mut() {
            let next_height = next_height_to_sequence.entry(*genesis).or_default();
            let first = *next_height;

            if ms.keys().next() != Some(&first) {
                continue;
            }
            // iterate over the keys, checking for contiguity and breaking at any gap
            'scan_keys: for (k1, k2) in ms
                .keys()
                .zip(ms.keys().skip(1).chain(std::iter::once(&i64::MAX)))
            {
                if k2 - k1 == 1 {
                } else {
                    // minimally, the first (contained) +1. or the last next-element
                    *next_height = k1 + 1;
                    break 'scan_keys;
                }
            }
            // go from the first (e.g. 0)  to the
            for k in first..*next_height {
                let e = ms.remove(&k).expect("Must be present");
                to_sequence.push_back((
                    e.header().height(),
                    e.canonicalized_hash_ref(),
                    e.get_genesis_hash(),
                ));
            }
        }

        info!(key=?keypair.x_only_public_key().0, n=to_sequence.len(), "Messages to Sequence");
        if !to_sequence.is_empty() {
            // to_sequence is naturally sorted by height here, which is a safe option.
            // However, for more fairness, we should randomize and ensure height sortedness later

            // ensure sorted by height
            let mut group_min_height = to_sequence[0].0;
            let mut g = to_sequence[0].2;
            // To sequence is grouped by genesis. It will look like:
            // g1@n g1@n+1 g2@m g3@n g3@n+1 g3@n+2
            // therefore we can detect groups by where the discontinuities are.
            // normalizes all the height groups...
            // g1@0 g1@1 g2@0 g3@0 g3@1 g3@2
            for s in to_sequence.iter_mut() {
                if s.2 == g {
                    s.0 -= group_min_height;
                } else {
                    group_min_height = s.0;
                    s.0 -= group_min_height;
                    g = s.2;
                }
            }

            let slice = to_sequence.make_contiguous();
            shuffle_slice(slice);

            // schedules one message per chain in a randomized round-robin,
            // balancing fairness and randomization.
            to_sequence.as_mut_slices().0.sort_by_key(|k| k.0);

            let msg = Channelized {
                data: BroadcastByHost::Sequence(to_sequence.iter().map(|k| k.1).collect()),
                channel: "default".into(),
            };
            {
                let mut handle = db.get_handle_all().await;
                // TODO: Run a tipcache

                // try to insert and handle
                let keypair = keypair;
                let secp = secp.clone();
                spawn_blocking(move || {
                    handle
                .retry_insert_authenticated_envelope_atomic::<Channelized<BroadcastByHost>, _, _>(
                    msg,
                    &keypair,
                    &secp,
                    None,
                    TipControl::GroupsOnly,
                )
                })
                .await??;
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

fn shuffle_slice<T>(to_sequence: &mut [T]) {
    let mut rng = rand::thread_rng();
    SliceRandom::shuffle(to_sequence, &mut rng);
}
