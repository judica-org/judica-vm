use attest_database::setup_db;
use attest_database::sql_error::SqliteFail;
use attest_database::{connection::MsgDB, db_handle::create::TipControl};
use attest_messages::{
    Authenticated, CanonicalEnvelopeHash, Envelope, GenericEnvelope, WrappedJson,
};
use game_host_messages::{BroadcastByHost, Channelized};
use sapio_bitcoin::secp256k1::rand;
use sapio_bitcoin::secp256k1::rand::seq::SliceRandom;
use sapio_bitcoin::secp256k1::All;
use sapio_bitcoin::{secp256k1::Secp256k1, KeyPair};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    error::Error,
    path::PathBuf,
    sync::Arc,
};
use tokio::spawn;
use tokio::task::JoinHandle;
use tor::TorConfig;
use tracing::{debug, info, warn};
mod app;
mod tor;
#[derive(Serialize, Deserialize)]
pub struct Config {
    tor: TorConfig,
    #[serde(default)]
    prefix: Option<PathBuf>,
    game_host_name: String,
}

fn get_config() -> Result<Arc<Config>, Box<dyn Error + Send + Sync>> {
    let config = std::env::var("GAME_HOST_CONFIG_JSON").map(|s| serde_json::from_str(&s))??;
    Ok(Arc::new(config))
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt::init();
    let config = get_config()?;
    let db = setup_db(
        &format!("attestations.{}", config.game_host_name),
        config.prefix.clone(),
    )
    .await
    .map_err(|e| format!("DB Setup Failed: {:?}", e))?;
    let tor_server = tor::start(config.clone()).await;

    let host = config.tor.get_hostname().await?;
    info!("Hosting Onion Service At: {}", host);

    let app_instance = app::run(config.clone(), db.clone());
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
        let handle = db.get_handle().await;
        let keymap = handle.get_keymap()?;
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
        let handle = db.get_handle().await;
        let v: Vec<Authenticated<GenericEnvelope<Channelized<BroadcastByHost>>>> =
            handle.load_all_messages_for_user_by_key_connected(&oracle_publickey)?;
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
        let handle = db.get_handle().await;
        handle
                .get_all_chain_commit_group_members_new_envelopes_for_chain_into_inconsistent::<Authenticated<Envelope>, WrappedJson>(
                    oracle_publickey,
                    &mut seq,
                    &mut all_unprocessed_messages,
                )?
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
            let handle = db.get_handle().await;
            handle
                .get_all_chain_commit_group_members_new_envelopes_for_chain_into_inconsistent::<Authenticated<Envelope>, WrappedJson>(
                    oracle_publickey,
                    &mut seq,
                    &mut all_unprocessed_messages,
                )?
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
            let mut handle = db.get_handle().await;
            // TODO: Run a tipcache

            // try to insert and handle
            handle
                .retry_insert_authenticated_envelope_atomic::<Channelized<BroadcastByHost>, _, _>(
                    msg,
                    &keypair,
                    &secp,
                    None,
                    TipControl::GroupsOnly,
                )?;
        }
    }
}

fn shuffle_slice<T>(to_sequence: &mut [T]) {
    let mut rng = rand::thread_rng();
    SliceRandom::shuffle(to_sequence, &mut rng);
}
