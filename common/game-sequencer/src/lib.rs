use attest_database::connection::MsgDB;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::Envelope;
use game_host_messages::Peer;
use game_host_messages::{BroadcastByHost, Channelized};
use mine_with_friends_board::game::game_move::GameMove;
use mine_with_friends_board::MoveEnvelope;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::XOnlyPublicKey;
use std::collections::hash_map::Entry::Occupied;
use std::collections::hash_map::Entry::Vacant;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio;
use tokio::spawn;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::sleep;

async fn make_sequenceing(db: MsgDB, oracle_publickey: XOnlyPublicKey) -> Option<Vec<Envelope>> {
    {
        let handle = db.get_handle().await;
        let v = handle
            .load_all_messages_for_user_by_key_connected(&oracle_publickey)
            .ok()?;
        let mut already_sequenced: VecDeque<CanonicalEnvelopeHash> = Default::default();
        for x in v {
            let d = serde_json::from_value::<Channelized<BroadcastByHost>>(x.msg().clone().into())
                .ok()?;
            match d.data {
                BroadcastByHost::Sequence(l) => already_sequenced.extend(l.iter()),
                BroadcastByHost::NewPeer(_) => {}
            }
        }
        let mut newer = None;
        let mut msgs = Default::default();
        handle
            .get_all_connected_messages_collect_into(&mut newer, &mut msgs)
            .ok()?;
        let all = already_sequenced
            .iter()
            .map(|h| msgs.remove(h))
            .collect::<Option<Vec<_>>>()?;
        let moves = all
            .iter()
            .map(|e| {
                Ok((
                    e.header().key(),
                    serde_json::from_value(e.msg().to_owned().into())?,
                ))
            })
            .collect::<Result<Vec<(XOnlyPublicKey, GameMove)>, serde_json::Error>>();
    }
    None
}

/// Goes through the oracles commitments in order
pub fn start_sequencer(
    shutdown: Arc<AtomicBool>,
    oracle_key: XOnlyPublicKey,
    db: MsgDB,
) -> (
    JoinHandle<()>,
    tokio::sync::mpsc::UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>,
) {
    let (mut tx, mut rx) = unbounded_channel();
    let task = spawn(async move {
        let mut count = 0;
        while !shutdown.load(Ordering::Relaxed) {
            'check: while !shutdown.load(Ordering::Relaxed) {
                let msg = {
                    let handle = db.get_handle().await;
                    handle.get_message_at_height_for_user(oracle_key, count)
                };
                match msg {
                    Ok(envelope) => {
                        match serde_json::from_value::<Channelized<BroadcastByHost>>(
                            envelope.msg().to_owned().into(),
                        ) {
                            Ok(v) => {
                                match v.data {
                                    BroadcastByHost::Sequence(s) => {
                                        if tx.send(s).is_err() {
                                            return;
                                        };
                                    }
                                    BroadcastByHost::NewPeer(Peer { service_url, port }) => {
                                        let handle = db.get_handle().await;
                                        // idempotent
                                        handle.insert_hidden_service(
                                            service_url,
                                            port,
                                            true,
                                            true,
                                            true,
                                        );
                                    }
                                }
                                count += 1;
                            }
                            Err(_) => {
                                return;
                            }
                        }
                        break 'check;
                    }
                    Err(_) => {
                        sleep(Duration::from_secs(10)).await;
                    }
                }
            }
        }
    });
    (task, rx)
}

/// This task builds a HashMap of all unprocessed envelopes regularly
pub fn start_envelope_db_fetcher(
    shutdown: Arc<AtomicBool>,
    db: MsgDB,
) -> (
    Arc<Mutex<HashMap<CanonicalEnvelopeHash, Envelope>>>,
    Arc<Notify>,
    JoinHandle<()>,
) {
    let shared_envelopes = Arc::new(Mutex::new(HashMap::<CanonicalEnvelopeHash, Envelope>::new()));
    let notify_new_envelopes = Arc::new(Notify::new());
    let envelopes = shared_envelopes.clone();
    let notify = notify_new_envelopes.clone();
    let task = spawn(async move {
        let mut newer = None;
        while !shutdown.load(Ordering::Relaxed) {
            let newer_before = newer;
            {
                let handle = db.get_handle().await;
                let mut env = envelopes.lock().await;
                handle.get_all_messages_collect_into_inconsistent(&mut newer, &mut env);
            }

            if newer_before != newer {
                notify.notify_waiters();
            }
            sleep(Duration::from_secs(10)).await;
        }
        notify.notify_waiters();
    });
    (shared_envelopes, notify_new_envelopes, task)
}

// Whenever new sequencing comes in, wait until they are all in the messages DB, and then drain them out for processing
pub fn start_envelope_batcher(
    shutdown: Arc<AtomicBool>,
    shared_envelopes: Arc<Mutex<HashMap<CanonicalEnvelopeHash, Envelope>>>,
    notify_new_envelopes: Arc<Notify>,
    mut input_envelope_hashers: UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>,
) -> (
    JoinHandle<()>,
    tokio::sync::mpsc::UnboundedReceiver<Envelope>,
) {
    let (mut tx_envelopes_to_process, mut rx_envelopes_to_process) = unbounded_channel();
    let envelopes = shared_envelopes.clone();
    let notify = notify_new_envelopes.clone();
    let task = spawn(async move {
        while let Some(mut envelope_hashes) = input_envelope_hashers.recv().await {
            let mut should_wait = None;
            'wait_for_new: while envelope_hashes.len() != 0 {
                if let Some(n) = should_wait.take() {
                    // register for notification, then drop lock, then wait
                    n.await;
                    // if we got woken up because of shutdown, shut down.
                    if shutdown.load(Ordering::Relaxed) {
                        return;
                    }
                }
                let mut envs = envelopes.lock().await;
                while let Some(envelope) = envelope_hashes.pop_front() {
                    match envs.entry(envelope) {
                        Occupied(e) => {
                            // TODO: Batch size
                            tx_envelopes_to_process.send(e.remove());
                        }
                        Vacant(k) => {
                            envelope_hashes.push_front(k.into_key());
                            should_wait.insert(notify.notified());
                            break 'wait_for_new;
                        }
                    }
                }
            }
        }
    });
    (task, rx_envelopes_to_process)
}

// Run the deserialization of the inner message type to move sets in it's own thread so that we can process
// moves in a pipeline as they get deserialized
// TODO: We skip invalid moves? Should do something else?
pub fn start_move_deserializer(
    shutdown: Arc<AtomicBool>,
    mut input_envelopes: tokio::sync::mpsc::UnboundedReceiver<Envelope>,
) -> (
    JoinHandle<()>,
    tokio::sync::mpsc::UnboundedReceiver<(MoveEnvelope, String)>,
) {
    let (mut tx2, mut rx2) = unbounded_channel();
    let task = spawn(async move {
        while let Some(envelope) = input_envelopes.recv().await {
            let r_game_move = serde_json::from_value(envelope.msg().to_owned().into());
            match r_game_move {
                Ok(game_move) => {
                    if tx2
                        .send((game_move, envelope.header().key().to_hex()))
                        .is_err()
                    {
                        return;
                    }
                }
                Err(_) => {}
            }
        }
    });
    (task, rx2)
}
