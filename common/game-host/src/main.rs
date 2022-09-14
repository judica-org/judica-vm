use attest_database::setup_db;
use attest_database::{connection::MsgDB, db_handle::create::TipControl};
use attest_messages::{Authenticated, CanonicalEnvelopeHash, Envelope};
use game_host_messages::{BroadcastByHost, Channelized};

use sapio_bitcoin::secp256k1::All;
use sapio_bitcoin::{
    secp256k1::{
        rand::{self},
        Secp256k1,
    },
    KeyPair, XOnlyPublicKey,
};
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
use tracing::{debug, info};
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
    let mut config = get_config()?;
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
        let handle = db.get_handle().await;
        let keymap = handle.get_keymap()?;

        for (key, value) in keymap {
            match task_set.entry(key) {
                std::collections::btree_map::Entry::Vacant(e) => {
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
                        let keypair = KeyPair::from_secret_key(&secp, &value);
                        let old = x.insert(spawn(game(
                            config.clone(),
                            db.clone(),
                            keypair,
                            secp.clone(),
                        )));
                        let res = old.await;
                        debug!(?res, ?key, "Game Task Quit");
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
    Ok(())
}

async fn game(
    config: Arc<Config>,
    db: MsgDB,
    keypair: KeyPair,
    secp: Arc<Secp256k1<All>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut seq = None;
    let oracle_publickey = keypair.public_key().x_only_public_key().0;
    let mut already_sequenced: Vec<CanonicalEnvelopeHash> = vec![];
    // First we get all of the old messages for the Oracle itself, so that we
    // can know which messages we've sequenced previously.
    {
        let handle = db.get_handle().await;
        let v = handle.load_all_messages_for_user_by_key_connected(&oracle_publickey)?;
        {
            for x in v {
                let d =
                    serde_json::from_value::<Channelized<BroadcastByHost>>(x.msg().clone().into())?;
                match d.data {
                    BroadcastByHost::Sequence(l) => already_sequenced.extend(l.iter()),
                    BroadcastByHost::NewPeer(_) => {}
                    BroadcastByHost::Heartbeat => {}
                    BroadcastByHost::GameSetup(_) => {}
                }
            }
        }
    }
    let mut all_unprocessed_messages = HashMap::new();
    let mut messages_by_user = HashMap::<XOnlyPublicKey, BTreeMap<i64, Envelope>>::new();
    let mut last_height_sequenced_for_user: HashMap<XOnlyPublicKey, Option<i64>> =
        Default::default();
    loop {
        // Get All the messages that we've not yet seen, but incosistently
        // Incosistency means that we may still be fetching priors.
        {
            let handle = db.get_handle().await;
            handle.get_all_messages_collect_into_inconsistent::<Authenticated<Envelope>>(
                &mut seq,
                &mut all_unprocessed_messages,
            )?
        }
        // Only on the first pass, remove the messages that have already been sequenced
        //
        // Later passes will be cleared.
        //
        // Also record the prior max sequenced.
        {
            for m in &already_sequenced {
                if let Some(msg) = all_unprocessed_messages.remove(m) {
                    let r = last_height_sequenced_for_user
                        .entry(msg.header().key())
                        .or_default();
                    *r = std::cmp::max(Some(msg.header().height()), *r);
                }
            }
            already_sequenced.clear();
        }
        //  Filter out events by the oracle and sort events by particular user
        {
            let unprocessed_message_keys =
                all_unprocessed_messages.keys().cloned().collect::<Vec<_>>();
            for value in &unprocessed_message_keys {
                // we can remove it now because the only reason we will drop it is if it is not to be sequenced
                if let Some((_k, e)) = all_unprocessed_messages.remove_entry(value) {
                    if e.header().key() != oracle_publickey {
                        if messages_by_user
                            .entry(e.header().key())
                            .or_default()
                            .insert(e.header().height(), e.inner())
                            .is_some()
                        {
                            // TODO: Panic?
                        }
                    }
                }
            }
        }
        // Sort the new entries
        let mut to_sequence = VecDeque::new();
        for (user, ms) in messages_by_user.iter_mut() {
            let height = last_height_sequenced_for_user.entry(*user).or_default();
            let mut next_height = match height {
                Some(u) => *u + 1,
                None => 0,
            };
            let first = next_height;
            // iterate over the keys, checking for contiguity and breaking at any gap
            for k in ms.keys() {
                if *k == next_height {
                    next_height += 1;
                } else {
                    break;
                }
            }
            // go from the first (e.g. 0)  to th
            for k in first..next_height {
                to_sequence.push_back(
                    ms.remove(&k)
                        .expect("Must be present")
                        .canonicalized_hash_ref(),
                );
            }
            // Set the next height
            *height = Some(next_height);
        }

        {
            let msg = ruma_serde::to_canonical_value(Channelized {
                data: BroadcastByHost::Sequence(to_sequence),
                channel: "default".into(),
            })?;
            let mut handle = db.get_handle().await;
            // TODO: Run a tipcache
            let wrapped = handle
                .wrap_message_in_envelope_for_user_by_key(
                    msg,
                    &keypair,
                    &secp,
                    None,
                    None,
                    TipControl::GroupsOnly,
                )??
                .self_authenticate(&secp)?;
            handle.try_insert_authenticated_envelope(wrapped)?;
        }
    }
}
