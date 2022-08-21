use attest_database::connection::MsgDB;
use attest_database::setup_db;
use attest_messages::{CanonicalEnvelopeHash, Envelope};
use game_host_messages::{BroadcastByHost, Channelized};
use sapio_bitcoin::{secp256k1::Secp256k1, KeyPair, XOnlyPublicKey};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    error::Error,
    sync::Arc,
};
use tor::TorConfig;
mod app;
mod tor;
pub struct Config {
    tor: TorConfig,
}

fn get_oracle_key() -> KeyPair {
    todo!()
}
fn get_config() -> Arc<Config> {
    let directory = todo!();
    let socks_port = todo!();
    let application_port = todo!();
    let application_path = todo!();
    Arc::new(Config {
        tor: TorConfig {
            directory,
            socks_port,
            application_port,
            application_path,
        },
    })
}
fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    let config = get_config();
    tor::start(config.clone());

    let v = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let db = setup_db("attestations.mining-game-host").await?;
            let app_instance = app::run(config, db.clone());
            let game_instance = game(db.clone());
            tokio::select! {
                a =  game_instance =>{
                    a?;
                },
                b = app_instance => {
                    b?.map_err(|e| format!("{}", e))?;
                }
            }
            Ok(())
        });
    v
}

async fn game(db: MsgDB) -> Result<(), Box<dyn Error>> {
    let secp = Secp256k1::new();
    let mut seq = None;
    let keypair = get_oracle_key();
    let oracle_publickey = keypair.public_key().x_only_public_key().0;
    let mut already_sequenced: Vec<CanonicalEnvelopeHash> = vec![];
    // First we get all of the old messages for the Oracle itself, so that we
    // can know which messages we've sequenced previously.
    {
        let handle = db.get_handle().await;
        if let Ok(v) = handle.load_all_messages_for_user_by_key(&oracle_publickey)? {
            for x in v {
                let d = serde_json::from_value::<Channelized<BroadcastByHost>>(x.msg)?;
                match d.data {
                    BroadcastByHost::Sequence(l) => already_sequenced.extend(l.iter()),
                    BroadcastByHost::NewPeer(_) => {}
                }
            }
        } else {
            panic!("Incosistent Database");
        }
    }
    let mut all_unprocessed_messages = HashMap::new();
    let mut messages_by_user = HashMap::<XOnlyPublicKey, BTreeMap<u64, Envelope>>::new();
    let mut last_height_sequenced_for_user: HashMap<XOnlyPublicKey, Option<u64>> =
        Default::default();
    loop {
        // Get All the messages that we've not yet seen, but incosistently
        // Incosistency means that we may still be fetching priors.
        {
            let handle = db.get_handle().await;
            handle.get_all_messages_collect_into_inconsistent(
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
                        .entry(msg.header.key)
                        .or_default();
                    *r = std::cmp::max(Some(msg.header.height), *r);
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
                    if e.header.key != oracle_publickey {
                        if messages_by_user
                            .entry(e.header.key)
                            .or_default()
                            .insert(e.header.height, e)
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
                        .canonicalized_hash_ref()
                        .unwrap(),
                );
            }
            // Set the next height
            *height = Some(next_height);
        }

        {
            let msg = serde_json::to_value(Channelized {
                data: BroadcastByHost::Sequence(to_sequence),
                channel: "default".into(),
            })?;
            let handle = db.get_handle().await;
            let wrapped = handle
                .wrap_message_in_envelope_for_user_by_key(msg, &keypair, &secp)??
                .self_authenticate(&secp)?;
            handle.try_insert_authenticated_envelope(wrapped)?;
        }
    }
}
