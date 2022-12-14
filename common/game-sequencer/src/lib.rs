// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(feature = "database_access")]
use attest_database::connection::MsgDB;
use attest_messages::AttestEnvelopable;
use attest_messages::Authenticated;
use attest_messages::AuthenticationError;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::GenericEnvelope;
use game_host_messages::Peer;
use game_host_messages::{BroadcastByHost, Channelized};
use sapio_bitcoin::secp256k1::Secp256k1;
use sapio_bitcoin::XOnlyPublicKey;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::fmt::Display;
use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
#[cfg(feature = "has_async")]
use tokio::{
    spawn,
    sync::{
        futures::Notified,
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex, Notify,
    },
    task::{spawn_blocking, JoinError, JoinHandle},
    time::sleep,
};
use tracing::debug;
use tracing::info;
use tracing::trace;
use tracing::warn;
pub mod game_specific;
pub use game_specific::*;

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(bound = "M: AttestEnvelopable")]
pub struct UnauthenticatedRawSequencer<M>
where
    M: AttestEnvelopable,
{
    pub sequencer_envelopes: Vec<GenericEnvelope<Channelized<BroadcastByHost>>>,
    pub msg_cache: HashMap<CanonicalEnvelopeHash, GenericEnvelope<M>>,
}
impl<M: AttestEnvelopable> TryFrom<UnauthenticatedRawSequencer<M>> for RawSequencer<M> {
    type Error = AuthenticationError;

    fn try_from(value: UnauthenticatedRawSequencer<M>) -> Result<Self, Self::Error> {
        Ok(Self {
            sequencer_envelopes: value
                .sequencer_envelopes
                .iter()
                .map(|v| v.solemnly_swear_self_authenticated())
                .collect::<Result<Vec<_>, AuthenticationError>>()?,
            msg_cache: value
                .msg_cache
                .iter()
                .map(|(m, e)| Ok((*m, e.solemnly_swear_self_authenticated()?)))
                .collect::<Result<HashMap<_, _>, AuthenticationError>>()?,
        })
    }
}

#[derive(Deserialize)]
#[serde(try_from = "UnauthenticatedRawSequencer<M>")]
#[serde(bound = "M: AttestEnvelopable")]
pub struct RawSequencer<M>
where
    M: AttestEnvelopable,
{
    pub sequencer_envelopes: Vec<Authenticated<GenericEnvelope<Channelized<BroadcastByHost>>>>,
    pub msg_cache: HashMap<CanonicalEnvelopeHash, Authenticated<GenericEnvelope<M>>>,
}

impl<M> JsonSchema for RawSequencer<M>
where
    M: AttestEnvelopable,
{
    fn schema_name() -> String {
        UnauthenticatedRawSequencer::<M>::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        UnauthenticatedRawSequencer::<M>::json_schema(gen)
    }
}

#[derive(Debug)]
pub enum SequencerError {
    BadMessageType,
    MessageFromWrongEntity,
    MissingTip,
    Gap,
    AuthenticationError,
}

impl Display for SequencerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for SequencerError {}

impl<M: AttestEnvelopable> TryFrom<RawSequencer<M>> for OfflineSequencer<M> {
    type Error = SequencerError;

    fn try_from(value: RawSequencer<M>) -> Result<Self, Self::Error> {
        if let Some(false) = value
            .sequencer_envelopes
            .first()
            .map(|v| v.header().height() == 0)
        {
            return Err(SequencerError::MissingTip);
        }
        if value
            .sequencer_envelopes
            .windows(2)
            .any(|s| s[0].header().height() + 1 != s[1].header().height())
        {
            return Err(SequencerError::Gap);
        }
        if value
            .sequencer_envelopes
            .windows(2)
            .any(|s| s[0].header().key() != s[1].header().key())
        {
            return Err(SequencerError::MessageFromWrongEntity);
        }
        let mut batches_to_sequence: Vec<VecDeque<CanonicalEnvelopeHash>> = vec![];
        for envelope in &value.sequencer_envelopes {
            let v = envelope.msg();
            match &v.data {
                BroadcastByHost::Sequence(s) => batches_to_sequence.push(s.clone()),
                BroadcastByHost::NewPeer(_) => {}
                BroadcastByHost::Heartbeat => {}
                BroadcastByHost::GameSetup(_) => {}
            }
        }
        Ok(OfflineSequencer {
            msg_cache: value.msg_cache,
            batches_to_sequence,
        })
    }
}

#[derive(Deserialize)]
#[serde(try_from = "RawSequencer<M>")]
#[serde(bound = "M: AttestEnvelopable")]
pub struct OfflineSequencer<M: AttestEnvelopable> {
    pub batches_to_sequence: Vec<VecDeque<CanonicalEnvelopeHash>>,
    pub msg_cache: HashMap<CanonicalEnvelopeHash, Authenticated<GenericEnvelope<M>>>,
}

impl<M: AttestEnvelopable> JsonSchema for OfflineSequencer<M> {
    fn schema_name() -> String {
        RawSequencer::<M>::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        RawSequencer::<M>::json_schema(gen)
    }
}
#[derive(Debug)]
pub enum SequenceingError<T> {
    MappingError(T),
    MissingEnvelope(CanonicalEnvelopeHash),
}

impl<E> From<E> for SequenceingError<E> {
    fn from(e: E) -> Self {
        SequenceingError::MappingError(e)
    }
}
impl<E: std::fmt::Debug> Display for SequenceingError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl<E> std::error::Error for SequenceingError<E> where E: std::error::Error {}

#[derive(Debug)]
pub enum Void {}
impl<M: AttestEnvelopable> OfflineSequencer<M> {
    pub fn directly_sequence(
        &mut self,
    ) -> Result<Vec<Authenticated<GenericEnvelope<M>>>, SequenceingError<Void>> {
        self.directly_sequence_map(|a| {
            Ok::<Option<Authenticated<GenericEnvelope<M>>>, Void>(Some(a))
        })
    }
    pub fn directly_sequence_map<F, R, E>(&mut self, f: F) -> Result<Vec<R>, SequenceingError<E>>
    where
        F: Fn(Authenticated<GenericEnvelope<M>>) -> Result<Option<R>, E>,
    {
        let mut v = vec![];
        for batch in &self.batches_to_sequence {
            for h in batch {
                if let Some(e) = self.msg_cache.remove(h) {
                    if let Some(r) = f(e)? {
                        v.push(r);
                    }
                } else {
                    return Err(SequenceingError::MissingEnvelope(*h));
                }
            }
        }

        Ok(v)
    }
}

#[cfg(feature = "has_async")]
impl<M: AttestEnvelopable> From<OfflineSequencer<M>> for OfflineDBFetcher<M> {
    fn from(o: OfflineSequencer<M>) -> Self {
        OfflineDBFetcher::new(o.batches_to_sequence, o.msg_cache)
    }
}

#[cfg(feature = "has_async")]
impl<M> TryFrom<OfflineDBFetcher<M>> for OfflineSequencer<M>
where
    M: AttestEnvelopable,
{
    type Error = ();

    fn try_from(value: OfflineDBFetcher<M>) -> Result<Self, Self::Error> {
        let mut batches = value.batches_to_sequence.try_lock().map_err(|_| ())?;
        let mut cache = value.msg_cache.try_lock().map_err(|_| ())?;
        let mut c = HashMap::default();
        let mut batches_to_sequence = Vec::default();
        std::mem::swap(&mut c, &mut cache);
        while let Ok(batch) = batches.try_recv() {
            batches_to_sequence.push(batch);
        }
        Ok(OfflineSequencer {
            batches_to_sequence,
            msg_cache: c,
        })
    }
}

#[cfg(feature = "has_async")]
pub struct OfflineDBFetcher<M: AttestEnvelopable> {
    batches_to_sequence: Arc<Mutex<UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>>>,
    msg_cache: Arc<Mutex<HashMap<CanonicalEnvelopeHash, Authenticated<GenericEnvelope<M>>>>>,
    new_msgs_in_cache: Arc<Notify>,
}

#[cfg(feature = "has_async")]
impl<M> OfflineDBFetcher<M>
where
    M: AttestEnvelopable,
{
    pub fn new(
        batches_to_sequence: Vec<VecDeque<CanonicalEnvelopeHash>>,
        msg_cache: HashMap<CanonicalEnvelopeHash, Authenticated<GenericEnvelope<M>>>,
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

    pub fn directly_sequence(
        self,
    ) -> Result<Vec<Authenticated<GenericEnvelope<M>>>, SequenceingError<()>> {
        OfflineSequencer::try_from(self)?.directly_sequence_map(|a| Ok(Some(a)))
    }
}
#[cfg(feature = "has_async")]
impl<M: AttestEnvelopable> DBFetcher<M> for OfflineDBFetcher<M> {
    fn batches_to_sequence(
        &self,
    ) -> Arc<Mutex<UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>>> {
        self.batches_to_sequence.clone()
    }

    fn msg_cache(
        &self,
    ) -> Arc<Mutex<HashMap<CanonicalEnvelopeHash, Authenticated<GenericEnvelope<M>>>>> {
        self.msg_cache.clone()
    }

    fn notified(&self) -> Notified<'_> {
        self.new_msgs_in_cache.notified()
    }
}

#[cfg(feature = "database_access")]
pub struct OnlineDBFetcher<M: AttestEnvelopable> {
    poll_sequencer_period: Duration,
    shutdown: Arc<AtomicBool>,
    db: MsgDB,
    schedule_batches_to_sequence: UnboundedSender<VecDeque<CanonicalEnvelopeHash>>,
    batches_to_sequence: Arc<Mutex<UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>>>,
    oracle_key: XOnlyPublicKey,
    msg_cache: Arc<Mutex<HashMap<CanonicalEnvelopeHash, Authenticated<GenericEnvelope<M>>>>>,
    rebuild_db_period: Duration,
    is_running: AtomicBool,
    new_msgs_in_cache: Arc<Notify>,
}
#[cfg(feature = "database_access")]
impl<M> OnlineDBFetcher<M>
where
    M: AttestEnvelopable + 'static,
{
    pub fn new(
        shutdown: Arc<AtomicBool>,
        poll_sequencer_period: Duration,
        rebuild_db_period: Duration,
        oracle_key: XOnlyPublicKey,
        db: MsgDB,
    ) -> Arc<Self> {
        let (schedule_batches_to_sequence, batches_to_sequence) = unbounded_channel();
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
            info!(key=?self.oracle_key, "Starting OnlineDBFetcher");
            let sequencer = self.clone().start_sequencer();
            let db_fetcher = self.clone().start_envelope_db_fetcher();
            sequencer.await.ok();
            db_fetcher.await.ok();
        }
    }
    fn should_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }
    /// Goes through the oracles commitments in order
    fn start_sequencer(self: Arc<Self>) -> JoinHandle<()> {
        spawn(async move {
            let mut count = 0;
            while !self.should_shutdown() {
                'check: while !self.should_shutdown() {
                    let msg: Result<
                        Option<Authenticated<GenericEnvelope<Channelized<BroadcastByHost>>>>,
                        _,
                    > = {
                        let handle = self.db.get_handle_read().await;
                        let oracle_key = self.oracle_key;
                        spawn_blocking(move || {
                            handle.get_message_at_height_for_user(oracle_key, count)
                        })
                        .await
                        .expect("Panic Free")
                    };
                    match msg {
                        Ok(None) => {
                            debug!(key=?self.oracle_key, sleep_for = ?self.poll_sequencer_period, "No New Messages Sleeping...");
                            sleep(self.poll_sequencer_period).await;
                            continue 'check;
                        }
                        Ok(Some(envelope)) => {
                            match &envelope.msg().data {
                                BroadcastByHost::Heartbeat => {}
                                BroadcastByHost::Sequence(s) => {
                                    info!(key=?self.oracle_key, n_msg = s.len(), "Got Batch to Sequence");
                                    if self.schedule_batches_to_sequence.send(s.clone()).is_err() {
                                        return;
                                    };
                                }
                                BroadcastByHost::NewPeer(Peer { service_url, port }) => {
                                    let handle = self.db.get_handle_all().await;
                                    let service_url = service_url.clone();
                                    let port = *port;
                                    spawn_blocking(move || {
                                        // idempotent
                                        handle
                                            .insert_hidden_service(
                                                service_url,
                                                port,
                                                true,
                                                true,
                                                true,
                                            )
                                            .ok();
                                    })
                                    .await;
                                }
                                BroadcastByHost::GameSetup(_) => {}
                            }
                            count += 1;
                        }
                        Err(e) => {
                            warn!(error=?e, "Database Failure")
                        }
                    }
                }
            }
        })
    }
    /// This task builds a HashMap of all unprocessed envelopes regularly
    fn start_envelope_db_fetcher(self: Arc<Self>) -> JoinHandle<()> {
        spawn(async move {
            let mut newer = None;
            while !self.should_shutdown() {
                let newer_before = newer;
                {
                    let mut env = self.msg_cache.clone().lock_owned().await;
                    let handle = self.db.get_handle_read().await;
                    // it's fine for us to filter for *only* game moves in our
                    // DB...  Ideally, we'd be able to use the group filter as
                    // well, but currently we don't have that set up properly
                    // for the host, so instead we load all moves.
                    let mut newer_copy = newer;
                    let (res, newer_overwrite) = spawn_blocking(move || {
                        let c = handle.get_all_messages_collect_into_inconsistent_skip_invalid(
                            &mut newer_copy,
                            &mut env,
                            true,
                        );

                        (c, newer_copy)
                    })
                    .await
                    .expect("Panic Free");
                    newer = newer_overwrite;

                    if let Err(e) = res {
                        warn!(error=?e, "DB Fetching Failed");
                        return;
                    }
                }

                if newer_before != newer {
                    info!(key=?self.oracle_key, new=newer, before=newer_before, "Got New Messages");
                    self.new_msgs_in_cache.notify_waiters();
                }
                debug!(key=?self.oracle_key, wait_till=?self.rebuild_db_period, "Waiting to scan DB Again");
                sleep(self.rebuild_db_period).await;
            }
            self.new_msgs_in_cache.notify_waiters();
        })
    }
}

#[cfg(feature = "database_access")]
impl<M: AttestEnvelopable> DBFetcher<M> for OnlineDBFetcher<M> {
    fn batches_to_sequence(
        &self,
    ) -> Arc<Mutex<UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>>> {
        self.batches_to_sequence.clone()
    }
    fn msg_cache(
        &self,
    ) -> Arc<Mutex<HashMap<CanonicalEnvelopeHash, Authenticated<GenericEnvelope<M>>>>> {
        self.msg_cache.clone()
    }

    fn notified(&self) -> Notified<'_> {
        self.new_msgs_in_cache.notified()
    }
}

#[cfg(feature = "has_async")]
pub trait DBFetcher<M>: Send + Sync
where
    M: AttestEnvelopable,
{
    fn batches_to_sequence(&self)
        -> Arc<Mutex<UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>>>;
    fn msg_cache(
        &self,
    ) -> Arc<Mutex<HashMap<CanonicalEnvelopeHash, Authenticated<GenericEnvelope<M>>>>>;
    fn notified(&self) -> Notified<'_>;
}
#[cfg(feature = "has_async")]
pub struct GenericSequencer<F, R, E, M: AttestEnvelopable> {
    db_fetcher: Arc<dyn DBFetcher<M>>,
    shutdown: Arc<AtomicBool>,
    push_next_envelope: UnboundedSender<Authenticated<GenericEnvelope<M>>>,
    output_envelope: Mutex<UnboundedReceiver<Authenticated<GenericEnvelope<M>>>>,
    push_next_move: UnboundedSender<R>,
    output_move: Mutex<UnboundedReceiver<R>>,
    is_running: AtomicBool,
    envelope_extractor: F,
    _pd: PhantomData<E>,
}

#[cfg(feature = "has_async")]
impl<F, R, E, M> GenericSequencer<F, R, E, M>
where
    F: Fn(Authenticated<GenericEnvelope<M>>) -> Result<R, E> + Send + Sync + 'static,
    E: Sync + Send + 'static + std::fmt::Debug,
    R: Send + 'static,
    M: AttestEnvelopable + 'static,
{
    pub fn new(
        shutdown: Arc<AtomicBool>,
        db_fetcher: Arc<dyn DBFetcher<M>>,
        envelope_extractor: F,
    ) -> Arc<Self> {
        let (push_next_envelope, output_envelope) = unbounded_channel();
        let output_envelope = Mutex::new(output_envelope);
        let (push_next_move, output_move) = unbounded_channel();
        let output_move = Mutex::new(output_move);
        Arc::new(Self {
            db_fetcher,
            shutdown,
            push_next_envelope,
            output_envelope,
            push_next_move,
            output_move,
            is_running: Default::default(),
            envelope_extractor,
            _pd: Default::default(),
        })
    }

    pub async fn run(self: Arc<Self>) -> Result<(), JoinError> {
        let last =
            self.is_running
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);
        if let Ok(false) = last {
            info!("Starting GenericSequencer");
            let batcher = self.clone().start_envelope_batcher();
            let move_deserializer = self.clone().start_move_deserializer();
            batcher.await?;
            move_deserializer.await?;
        }
        Ok(())
    }

    pub async fn output_move(self: &Arc<Self>) -> Option<R> {
        self.output_move.lock().await.recv().await
    }

    fn should_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }

    // Whenever new sequencing comes in, wait until they are all in the messages DB, and then drain them out for processing
    fn start_envelope_batcher(self: Arc<Self>) -> JoinHandle<()> {
        spawn(async move {
            let batches = self.db_fetcher.batches_to_sequence();
            let mut input_envelope_hashes = batches.lock().await;
            let msg_cache = self.db_fetcher.msg_cache();
            while let Some(mut envelope_hashes) = input_envelope_hashes.recv().await {
                info!(n = envelope_hashes.len(), "Got New Batch to Sequence");
                let mut should_wait = None;
                'wait_for_new: while !envelope_hashes.is_empty() {
                    if let Some(n) = should_wait.take() {
                        // register for notification, then drop lock, then wait
                        n.await;
                        trace!("Awoken from Waiting");
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
                                let envelope = e.remove();
                                if self.push_next_envelope.send(envelope).is_err() {
                                    // quit if the channel is closed
                                    return;
                                }
                            }
                            Vacant(k) => {
                                let msg_hash = k.into_key();
                                envelope_hashes.push_front(msg_hash);
                                should_wait = Some(self.db_fetcher.notified());
                                info!(?msg_hash, "Wait for Envelope");
                                continue 'wait_for_new;
                            }
                        }
                    }
                }
            }
        })
    }
    // Run the deserialization of the inner message type to move sets in it's own thread so that we can process
    // moves in a pipeline as they get deserialized
    // TODO: We skip invalid moves? Should do something else?
    fn start_move_deserializer(self: Arc<Self>) -> JoinHandle<()> {
        spawn(async move {
            let mut next_envelope = self.output_envelope.lock().await;
            while let Some(envelope) = next_envelope.recv().await {
                trace!(msg_hash=?envelope.canonicalized_hash_ref(), "Got Envelope");
                let extracted = (self.envelope_extractor)(envelope);
                match extracted {
                    Ok(extracted) => {
                        if self.push_next_move.send(extracted).is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        // Skip
                        warn!(bad_move=?e,"Invalid Move Found");
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod test {
    use crate::game_specific::Sequencer;

    use super::*;
    use attest_messages::{nonce::PrecomittedNonce, Header, Unsigned};
    use game_player_messages::ParticipantAction;
    use mine_with_friends_board::{
        game::game_move::{GameMove, Heartbeat},
        sanitize::Unsanitized,
    };
    use sapio_bitcoin::{
        secp256k1::{rand, SecretKey},
        KeyPair,
    };
    use std::collections::VecDeque;

    fn make_random_moves() -> Vec<VecDeque<Authenticated<GenericEnvelope<ParticipantAction>>>> {
        let secp = &sapio_bitcoin::secp256k1::Secp256k1::new();

        (0..10)
            .map(|_j| {
                (0..100)
                    .map(|_i| {
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

                        let mut e1 = GenericEnvelope::new(
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
                            ParticipantAction::new(
                                Unsanitized(GameMove::Heartbeat(Heartbeat())),
                                1,
                                sent_time_ms as u64,
                            ),
                        );
                        e1.sign_with(
                            &KeyPair::from_secret_key(secp, &sk),
                            secp,
                            PrecomittedNonce::new(secp),
                        )
                        .expect("Signature OK");
                        e1.self_authenticate(secp).expect("Must Be Correct")
                    })
                    .collect::<VecDeque<_>>()
            })
            .collect::<Vec<_>>()
    }
    #[tokio::test]
    async fn test_offline_sequencer() {
        let envelopes = make_random_moves();
        let hmap = envelopes
            .iter()
            .flat_map(|e| e.iter())
            .map(|e| (e.canonicalized_hash_ref(), e.clone()))
            .collect::<HashMap<_, _>>();
        let hashes = envelopes
            .iter()
            .map(|es| es.iter().map(|e| e.canonicalized_hash_ref()).collect())
            .collect::<Vec<_>>();
        let db_fetcher_direct = OfflineDBFetcher::new(hashes.clone(), hmap.clone())
            .directly_sequence()
            .unwrap();
        let db_fetcher = Arc::new(OfflineDBFetcher::new(hashes, hmap));
        let s = Sequencer::new(Default::default(), db_fetcher);
        {
            let s = s.clone();
            spawn(async move { s.run().await });
        }

        assert!(envelopes.iter().flatten().eq(db_fetcher_direct.iter()));

        for batch in envelopes {
            for envelope in batch {
                if let Some((m, x)) = s.output_move().await {
                    let game_move = envelope.msg();
                    assert_eq!(ParticipantAction::MoveEnvelope(m), *game_move);
                    assert_eq!(x, envelope.header().key());
                } else {
                    unreachable!("Offline GenericSequencer did not sequence all messages")
                }
            }
        }
    }
    struct TestDBFetcher {
        to_seq: Vec<VecDeque<Authenticated<GenericEnvelope<ParticipantAction>>>>,
        schedule_batches_to_sequence: UnboundedSender<VecDeque<CanonicalEnvelopeHash>>,
        batches_to_sequence: Arc<Mutex<UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>>>,
        msg_cache: Arc<
            Mutex<
                HashMap<CanonicalEnvelopeHash, Authenticated<GenericEnvelope<ParticipantAction>>>,
            >,
        >,
        new_msgs_in_cache: Arc<Notify>,
    }
    impl TestDBFetcher {
        pub fn new(
            to_seq: Vec<VecDeque<Authenticated<GenericEnvelope<ParticipantAction>>>>,
        ) -> Arc<Self> {
            let (schedule_batches_to_sequence, batches_to_sequence) = unbounded_channel();
            let batches_to_sequence = Arc::new(Mutex::new(batches_to_sequence));
            Arc::new(Self {
                schedule_batches_to_sequence,
                batches_to_sequence,
                msg_cache: Default::default(),
                new_msgs_in_cache: Default::default(),
                to_seq,
            })
        }
        async fn run(self: Arc<Self>) {
            spawn({
                let me = Arc::clone(&self);
                async move {
                    for batch in &me.to_seq {
                        if me
                            .schedule_batches_to_sequence
                            .send(batch.iter().map(|e| e.canonicalized_hash_ref()).collect())
                            .is_err()
                        {
                            // quit on close
                            return;
                        }
                        tokio::task::yield_now().await;
                    }
                }
            });
            let me = Arc::clone(&self);
            let mut all: VecDeque<_> = me.to_seq.iter().flatten().collect();
            let cache = &Arc::clone(&self.msg_cache);
            loop {
                let send_later: Vec<_> = (0..5)
                    .flat_map(|_| all.pop_front())
                    .map(|e| (e.canonicalized_hash_ref(), e.clone()))
                    .collect();
                let send_now: Vec<_> = (0..5)
                    .flat_map(|_| all.pop_front())
                    .map(|e| (e.canonicalized_hash_ref(), e.clone()))
                    .collect();
                let quit = send_later.len() < 5 || send_now.len() < 5;
                if send_later.len() + send_now.len() == 0 {
                    break;
                }

                {
                    let cache = Arc::clone(cache);
                    let mut cache = cache.lock().await;
                    for (k, v) in send_now {
                        cache.insert(k, v);
                    }
                }
                me.new_msgs_in_cache.notify_waiters();
                spawn({
                    let cache = Arc::clone(cache);
                    let me = me.clone();
                    async move {
                        tokio::task::yield_now().await;
                        {
                            let mut cache = cache.lock().await;
                            for (k, v) in send_later {
                                cache.insert(k, v);
                            }
                        }
                        me.new_msgs_in_cache.notify_waiters();
                    }
                });
                tokio::task::yield_now().await;

                if quit {
                    break;
                }
            }
        }
    }
    impl DBFetcher<ParticipantAction> for TestDBFetcher {
        fn batches_to_sequence(
            &self,
        ) -> Arc<Mutex<UnboundedReceiver<VecDeque<CanonicalEnvelopeHash>>>> {
            self.batches_to_sequence.clone()
        }

        fn msg_cache(
            &self,
        ) -> Arc<
            Mutex<
                HashMap<CanonicalEnvelopeHash, Authenticated<GenericEnvelope<ParticipantAction>>>,
            >,
        > {
            self.msg_cache.clone()
        }

        fn notified(&self) -> Notified<'_> {
            self.new_msgs_in_cache.notified()
        }
    }
    #[tokio::test]
    async fn test_periodic_seqeuencer() {
        let envelopes = make_random_moves();
        let db_fetcher = TestDBFetcher::new(envelopes.clone());
        spawn(db_fetcher.clone().run());
        let s = Sequencer::new(Default::default(), db_fetcher);
        {
            let s = s.clone();
            spawn(async move { s.run().await });
        }
        for batch in envelopes {
            for envelope in batch {
                if let Some((m, x)) = s.output_move().await {
                    let game_move = envelope.msg();
                    assert_eq!(ParticipantAction::MoveEnvelope(m), *game_move);
                    assert_eq!(x, envelope.header().key());
                } else {
                    unreachable!("Online GenericSequencer did not sequence all messages")
                }
            }
        }
    }
}
