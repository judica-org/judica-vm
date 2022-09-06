#[cfg(feature = "database_access")]
use attest_database::connection::MsgDB;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::Envelope;
use game_host_messages::Peer;
use game_host_messages::{BroadcastByHost, Channelized};
use mine_with_friends_board::game::game_move::GameMove;
use mine_with_friends_board::MoveEnvelope;
use sapio_bitcoin::XOnlyPublicKey;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::future;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio;
use tokio::spawn;
use tokio::sync::futures::Notified;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    Mutex, Notify,
};
use tokio::task::JoinHandle;
use tokio::time::sleep;

// TODO: Examine this logic
// async fn make_sequenceing(db: MsgDB, oracle_publickey: XOnlyPublicKey) -> Option<Vec<Envelope>> {
//     {
//         let handle = db.get_handle().await;
//         let v = handle
//             .load_all_messages_for_user_by_key_connected(&oracle_publickey)
//             .ok()?;
//         let mut already_sequenced: VecDeque<CanonicalEnvelopeHash> = Default::default();
//         for x in v {
//             let d = serde_json::from_value::<Channelized<BroadcastByHost>>(x.msg().clone().into())
//                 .ok()?;
//             match d.data {
//                 BroadcastByHost::Sequence(l) => already_sequenced.extend(l.iter()),
//                 BroadcastByHost::NewPeer(_) => {}
//             }
//         }
//         let mut newer = None;
//         let mut msgs = Default::default();
//         handle
//             .get_all_connected_messages_collect_into(&mut newer, &mut msgs)
//             .ok()?;
//         let all = already_sequenced
//             .iter()
//             .map(|h| msgs.remove(h))
//             .collect::<Option<Vec<_>>>()?;
//         let moves = all
//             .iter()
//             .map(|e| {
//                 Ok((
//                     e.header().key(),
//                     serde_json::from_value(e.msg().to_owned().into())?,
//                 ))
//             })
//             .collect::<Result<Vec<(XOnlyPublicKey, GameMove)>, serde_json::Error>>();
//     }
//     None
// }

pub struct OfflineDBFetcher {
    batches_to_sequence: Arc<Mutex<UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>>>,
    msg_cache: Arc<Mutex<HashMap<CanonicalEnvelopeHash, Envelope>>>,
    new_msgs_in_cache: Arc<Notify>,
}

impl OfflineDBFetcher {
    pub fn new(
        batches_to_sequence: Vec<VecDeque<CanonicalEnvelopeHash>>,
        msg_cache: HashMap<CanonicalEnvelopeHash, Envelope>,
    ) -> Self {
        let (tx, rx) = unbounded_channel();
        for batch in batches_to_sequence {
            tx.send(batch).expect("Always Open");
        }
        Self {
            batches_to_sequence: Arc::new(Mutex::new(rx)),
            msg_cache: Arc::new(Mutex::new(msg_cache)),
            new_msgs_in_cache: Default::default(),
        }
    }
}
impl DBFetcher for OfflineDBFetcher {
    fn batches_to_sequence(
        &self,
    ) -> Arc<Mutex<UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>>> {
        self.batches_to_sequence.clone()
    }

    fn msg_cache(&self) -> Arc<Mutex<HashMap<CanonicalEnvelopeHash, Envelope>>> {
        self.msg_cache.clone()
    }

    fn notified(&self) -> Notified<'_> {
        self.new_msgs_in_cache.notified()
    }
}

#[cfg(feature = "database_access")]
pub struct OnlineDBFetcher {
    poll_sequencer_period: Duration,
    shutdown: Arc<AtomicBool>,
    db: MsgDB,
    schedule_batches_to_sequence: UnboundedSender<VecDeque<CanonicalEnvelopeHash>>,
    batches_to_sequence: Arc<Mutex<UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>>>,
    oracle_key: XOnlyPublicKey,
    msg_cache: Arc<Mutex<HashMap<CanonicalEnvelopeHash, Envelope>>>,
    rebuild_db_period: Duration,
    is_running: AtomicBool,
    new_msgs_in_cache: Arc<Notify>,
}
#[cfg(feature = "database_access")]
impl OnlineDBFetcher {
    pub fn new(
        shutdown: Arc<AtomicBool>,
        poll_sequencer_period: Duration,
        rebuild_db_period: Duration,
        oracle_key: XOnlyPublicKey,
        db: MsgDB,
    ) -> Arc<Self> {
        let (schedule_batches_to_sequence, mut batches_to_sequence) = unbounded_channel();
        let batches_to_sequence = Arc::new(Mutex::new(batches_to_sequence));
        Arc::new(Self {
            poll_sequencer_period,
            shutdown,
            db,
            schedule_batches_to_sequence,
            batches_to_sequence,
            oracle_key,
            msg_cache: Default::default(),
            new_msgs_in_cache: Default::default(),
            rebuild_db_period,
            is_running: Default::default(),
        })
    }

    pub async fn run(self: Arc<Self>) {
        let last =
            self.is_running
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);
        if let Ok(false) = last {
            let sequencer = self.clone().start_sequencer();
            let db_fetcher = self.clone().start_envelope_db_fetcher();
            sequencer.await;
            db_fetcher.await;
        }
    }
    fn should_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }
    /// Goes through the oracles commitments in order
    fn start_sequencer(self: Arc<Self>) -> JoinHandle<()> {
        let task = spawn(async move {
            let mut count = 0;
            while !self.should_shutdown() {
                'check: while !self.should_shutdown() {
                    let msg = {
                        let handle = self.db.get_handle().await;
                        handle.get_message_at_height_for_user(self.oracle_key, count)
                    };
                    match msg {
                        Ok(envelope) => {
                            match serde_json::from_value::<Channelized<BroadcastByHost>>(
                                envelope.msg().to_owned().into(),
                            ) {
                                Ok(v) => {
                                    match v.data {
                                        BroadcastByHost::Sequence(s) => {
                                            if self.schedule_batches_to_sequence.send(s).is_err() {
                                                return;
                                            };
                                        }
                                        BroadcastByHost::NewPeer(Peer { service_url, port }) => {
                                            let handle = self.db.get_handle().await;
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
                            sleep(self.poll_sequencer_period).await;
                        }
                    }
                }
            }
        });
        task
    }
    /// This task builds a HashMap of all unprocessed envelopes regularly
    fn start_envelope_db_fetcher(self: Arc<Self>) -> (JoinHandle<()>) {
        let task = spawn(async move {
            let mut newer = None;
            while !self.should_shutdown() {
                let newer_before = newer;
                {
                    let handle = self.db.get_handle().await;
                    let mut env = self.msg_cache.lock().await;
                    handle.get_all_messages_collect_into_inconsistent(&mut newer, &mut env);
                }

                if newer_before != newer {
                    self.new_msgs_in_cache.notify_waiters();
                }
                sleep(self.rebuild_db_period).await;
            }
            self.new_msgs_in_cache.notify_waiters();
        });
        task
    }
}

#[cfg(feature = "database_access")]
impl DBFetcher for OnlineDBFetcher {
    fn batches_to_sequence(
        &self,
    ) -> Arc<Mutex<UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>>> {
        self.batches_to_sequence.clone()
    }
    fn msg_cache(&self) -> Arc<Mutex<HashMap<CanonicalEnvelopeHash, Envelope>>> {
        self.msg_cache.clone()
    }

    fn notified(&self) -> Notified<'_> {
        self.new_msgs_in_cache.notified()
    }
}

pub trait DBFetcher: Send + Sync {
    fn batches_to_sequence(&self)
        -> Arc<Mutex<UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>>>;
    fn msg_cache(&self) -> Arc<Mutex<HashMap<CanonicalEnvelopeHash, Envelope>>>;
    fn notified(&self) -> Notified<'_>;
}
pub struct Sequencer {
    db_fetcher: Arc<dyn DBFetcher>,
    shutdown: Arc<AtomicBool>,
    push_next_envelope: UnboundedSender<Envelope>,
    output_envelope: Mutex<UnboundedReceiver<Envelope>>,
    push_next_move: UnboundedSender<(MoveEnvelope, XOnlyPublicKey)>,
    output_move: Mutex<UnboundedReceiver<(MoveEnvelope, XOnlyPublicKey)>>,
    is_running: AtomicBool,
}

impl Sequencer {
    pub fn new(shutdown: Arc<AtomicBool>, db_fetcher: Arc<dyn DBFetcher>) -> Arc<Self> {
        let (push_next_envelope, output_envelope) = unbounded_channel();
        let output_envelope = Mutex::new(output_envelope);
        let (push_next_move, output_move) = unbounded_channel();
        let output_move = Mutex::new(output_move);
        Arc::new(Sequencer {
            db_fetcher,
            shutdown,
            push_next_envelope,
            output_envelope,
            push_next_move,
            output_move,
            is_running: Default::default(),
        })
    }

    pub async fn run(self: Arc<Self>) {
        let last =
            self.is_running
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);
        if let Ok(false) = last {
            let batcher = self.clone().start_envelope_batcher();
            let move_deserializer = self.clone().start_move_deserializer();
            batcher.await;
            move_deserializer.await;
        }
    }

    pub async fn output_move(self: &Arc<Self>) -> Option<(MoveEnvelope, XOnlyPublicKey)> {
        self.output_move.lock().await.recv().await
    }

    fn should_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    // Whenever new sequencing comes in, wait until they are all in the messages DB, and then drain them out for processing
    fn start_envelope_batcher(self: Arc<Self>) -> JoinHandle<()> {
        let task = spawn(async move {
            let batches = self.db_fetcher.batches_to_sequence();
            let mut input_envelope_hashes = batches.lock().await;
            let msg_cache = self.db_fetcher.msg_cache();
            while let Some(mut envelope_hashes) = input_envelope_hashes.recv().await {
                let mut should_wait = None;
                'wait_for_new: while envelope_hashes.len() != 0 {
                    if let Some(n) = should_wait.take() {
                        // register for notification, then drop lock, then wait
                        n.await;
                        // if we got woken up because of shutdown, shut down.
                        if self.should_shutdown() {
                            return;
                        }
                    }
                    let mut envs = msg_cache.lock().await;
                    while let Some(envelope) = envelope_hashes.pop_front() {
                        match envs.entry(envelope) {
                            Occupied(e) => {
                                // TODO: Batch size
                                self.push_next_envelope.send(e.remove());
                            }
                            Vacant(k) => {
                                envelope_hashes.push_front(k.into_key());
                                should_wait.insert(self.db_fetcher.notified());
                                break 'wait_for_new;
                            }
                        }
                    }
                }
            }
        });
        task
    }
    // Run the deserialization of the inner message type to move sets in it's own thread so that we can process
    // moves in a pipeline as they get deserialized
    // TODO: We skip invalid moves? Should do something else?
    fn start_move_deserializer(self: Arc<Self>) -> (JoinHandle<()>) {
        let task = spawn(async move {
            let mut next_envelope = self.output_envelope.lock().await;
            while let Some(envelope) = next_envelope.recv().await {
                let r_game_move = serde_json::from_value(envelope.msg().to_owned().into());
                match r_game_move {
                    Ok(game_move) => {
                        if self
                            .push_next_move
                            .send((game_move, envelope.header().key()))
                            .is_err()
                        {
                            return;
                        }
                    }
                    Err(_) => {}
                }
            }
        });
        task
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use attest_messages::{nonce::PrecomittedNonce, Envelope, Header, Unsigned};
    use mine_with_friends_board::{entity::EntityID, game::game_move::Init, sanitize::Unsanitized};
    use ruma_serde::CanonicalJsonValue;
    use sapio_bitcoin::secp256k1::{rand, SecretKey};
    use std::collections::VecDeque;

    #[tokio::test]
    async fn test_offline_sequencer() {
        let secp = &sapio_bitcoin::secp256k1::Secp256k1::new();
        let envelopes = (0..10)
            .map(|j| {
                (0..100)
                    .map(|i| {
                        let next_nonce_s = PrecomittedNonce::new(secp);
                        let next_nonce = next_nonce_s.get_public(secp);
                        let sk = SecretKey::new(&mut rand::thread_rng());
                        let key = sk.x_only_public_key(secp).0;

                        let ancestors = None;
                        let tips = vec![];
                        let sent_time_ms = 12431;
                        let unsigned = Unsigned::new(None);
                        let checkpoints = Default::default();
                        let height = 0;

                        let e1 = Envelope::new(
                            Header::new(
                                key,
                                next_nonce,
                                ancestors,
                                tips,
                                height,
                                sent_time_ms,
                                unsigned,
                                checkpoints,
                            ),
                            ruma_serde::to_canonical_value(MoveEnvelope::new(
                                Unsanitized(GameMove::Init(Init())),
                                1,
                                EntityID((j<<10) + i),
                                sent_time_ms as u64,
                            ))
                            .unwrap(),
                        );
                        e1
                    })
                    .collect::<VecDeque<_>>()
            })
            .collect::<Vec<_>>();

        let hmap = envelopes
            .iter()
            .map(|e| e.iter())
            .flatten()
            .map(|e| (e.canonicalized_hash_ref(), e.clone()))
            .collect();
        let hashes = envelopes
            .iter()
            .map(|es| es.iter().map(|e| e.canonicalized_hash_ref()).collect())
            .collect();
        let db_fetcher = Arc::new(OfflineDBFetcher::new(hashes, hmap));
        let s = Sequencer::new(Default::default(), db_fetcher);
        {
            let s = s.clone();
            spawn(async {
                s.run().await;
            });
        }
        for batch in envelopes {
            for envelope in batch {
                if let Some((m, x)) = s.output_move().await {
                    let game_move = serde_json::from_value(envelope.msg().clone().into()).unwrap();
                    assert_eq!(m, game_move);
                    assert_eq!(x, envelope.header().key());
                } else {
                    assert!(false);
                }
            }
        }
    }
}
