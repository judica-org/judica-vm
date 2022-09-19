use super::handle_type;
use super::ChainCommitGroupID;
use super::MsgDBHandle;
use crate::db_handle::sql::insert::*;
use crate::sql_error;
use crate::sql_serializers::PK;
use crate::sql_serializers::SK;
use attest_messages::nonce::PrecomittedNonce;
use attest_messages::nonce::PrecomittedPublicNonce;
use attest_messages::AttestEnvelopable;
use attest_messages::Authenticated;
use attest_messages::CanonicalEnvelopeHash;

use attest_messages::GenericEnvelope;
use rusqlite::ffi;
use rusqlite::ffi::{SQLITE_CONSTRAINT_CHECK, SQLITE_CONSTRAINT_NOTNULL, SQLITE_CONSTRAINT_UNIQUE};
use rusqlite::params;
use rusqlite::ErrorCode;
use rusqlite::Transaction;
use sapio_bitcoin::secp256k1::rand::thread_rng;
use sapio_bitcoin::secp256k1::rand::Rng;
use sapio_bitcoin::{
    hashes::hex::ToHex,
    secp256k1::{Secp256k1, Signing},
    KeyPair, XOnlyPublicKey,
};
use tracing::debug;
use tracing::info;
use tracing::trace;

impl<'a, T> MsgDBHandle<'a, T>
where
    T: handle_type::Insert,
{
    /// Creates a new random nonce and saves it for the given user.
    pub fn generate_fresh_nonce_for_user_by_key<C: Signing>(
        &self,
        secp: &Secp256k1<C>,
        key: XOnlyPublicKey,
    ) -> Result<PrecomittedPublicNonce, rusqlite::Error> {
        let nonce = PrecomittedNonce::new(secp);
        let pk_nonce = self.save_nonce_for_user_by_key(nonce, secp, key)?;
        Ok(pk_nonce)
    }
    /// Saves an arbitrary nonce for the given user.
    pub fn save_nonce_for_user_by_key<C: Signing>(
        &self,
        nonce: PrecomittedNonce,
        secp: &Secp256k1<C>,
        key: XOnlyPublicKey,
    ) -> Result<PrecomittedPublicNonce, rusqlite::Error> {
        let pk_nonce = nonce.get_public(secp);
        let mut stmt = self.0.prepare_cached(SQL_INSERT_NONCE_BY_KEY)?;
        stmt.insert(rusqlite::params![PK(key), pk_nonce, nonce,])?;
        Ok(pk_nonce)
    }

    /// adds a hidden service to our connection list
    /// Won't fail if already exists
    pub fn insert_hidden_service(
        &self,
        s: String,
        port: u16,
        fetch_from: bool,
        push_to: bool,
        allow_unsolicited_tips: bool,
    ) -> Result<(), rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_INSERT_HIDDEN_SERVICE)?;
        stmt.insert(rusqlite::named_params! {
        ":service_url": s,
        ":port": port,
        ":fetch_from":fetch_from,
        ":push_to": push_to,
        ":allow_unsolicited_tips": allow_unsolicited_tips})?;
        Ok(())
    }

    /// saves a keypair to our keyset
    pub fn save_keypair(&self, kp: KeyPair) -> Result<(), rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_INSERT_KEYPAIR)?;

        stmt.insert(rusqlite::params![
            PK(kp.x_only_public_key().0),
            SK(kp.secret_key())
        ])?;
        Ok(())
    }

    /// creates a new user from a genesis envelope
    #[must_use = "Must Check that the new user was succesfully created"]
    pub fn insert_user_by_genesis_envelope<M>(
        &mut self,
        nickname: String,
        envelope: Authenticated<GenericEnvelope<M>>,
    ) -> Result<Result<String, (sql_error::SqliteFail, Option<String>)>, rusqlite::Error>
    where
        M: AttestEnvelopable,
    {
        info!(genesis=?envelope.get_genesis_hash(), nickname, "Creating New Genesis");
        let tx = self.0.transaction()?;
        let mut stmt = tx.prepare_cached(SQL_INSERT_USER)?;
        let hex_key = PK(envelope.header().key());
        match stmt.insert(params![nickname, hex_key]) {
            Ok(_rowid) => {
                tracing::info!(?nickname, key=?hex_key.0, "Successfully Created New User");
            }
            Err(e) => match e {
                rusqlite::Error::SqliteFailure(
                    ffi::Error {
                        code: ErrorCode::ConstraintViolation,
                        extended_code: SQLITE_CONSTRAINT_UNIQUE,
                    },
                    msg,
                ) => {
                    debug!(key=?hex_key.0, err=msg, "A User with this key already exists...");
                    // Don't Care -- Insert Envelope Anyway
                }
                other_err => {
                    debug!(
                        ?other_err,
                        "SQL: {}",
                        stmt.expanded_sql().unwrap_or_default()
                    );
                    return Err(other_err);
                }
            },
        }
        let res = try_insert_authenticated_envelope_with_txn(envelope, &tx)
            .map(|t| t.and(Ok(hex_key.0.to_hex())));
        drop(stmt);
        tx.commit()?;
        res
    }
    /// attempts to put an authenticated envelope in the DB
    ///
    /// Will fail if the key is not registered.
    ///
    /// Will return false if the message already existed
    #[must_use = "Required to check if the insertion of an Envelope was successful"]
    pub fn try_insert_authenticated_envelope<M>(
        &mut self,
        data: Authenticated<GenericEnvelope<M>>,
    ) -> Result<Result<(), (sql_error::SqliteFail, Option<String>)>, rusqlite::Error>
    where
        M: AttestEnvelopable,
    {
        let tx = self.0.transaction()?;
        let res = try_insert_authenticated_envelope_with_txn(data, &tx);
        tx.commit()?;
        res
    }

    /// Create a new Chain Commit Group
    pub fn new_chain_commit_group(
        &self,
        name: Option<String>,
    ) -> Result<(String, ChainCommitGroupID), rusqlite::Error> {
        let name = name.unwrap_or_else(|| {
            let mut r = thread_rng();
            let u: [u8; 32] = r.gen::<[u8; 32]>();
            u.to_hex()
        });
        let mut stmt = self.0.prepare_cached(SQL_INSERT_CHAIN_COMMIT_GROUP)?;
        let i = stmt.insert(rusqlite::named_params!(":name": name))?;
        Ok((name, ChainCommitGroupID(i)))
    }

    /// Add Member to Chain Commit Group
    pub fn add_member_to_chain_commit_group(
        &self,
        group_id: ChainCommitGroupID,
        genesis_hash: CanonicalEnvelopeHash,
    ) -> Result<(), rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(SQL_INSERT_CHAIN_COMMIT_GROUP_MEMBER)?;
        let _ = stmt.insert(rusqlite::named_params!(
            ":genesis_hash": genesis_hash,
            ":group_id": group_id
        ))?;
        Ok(())
    }
    /// Add Member to Chain Commit Group
    pub fn add_subscriber_to_chain_commit_group(
        &self,
        group_id: ChainCommitGroupID,
        genesis_hash: CanonicalEnvelopeHash,
    ) -> Result<(), rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(SQL_INSERT_CHAIN_COMMIT_GROUP_SUBSCRIBER)?;
        let _ = stmt.insert(rusqlite::named_params!(
            ":genesis_hash": genesis_hash,
            ":group_id": group_id
        ))?;
        Ok(())
    }
}

#[must_use = "Required to check if the insertion of an Envelope was successful"]
pub fn try_insert_authenticated_envelope_with_txn<M>(
    data: Authenticated<GenericEnvelope<M>>,
    tx: &Transaction,
) -> Result<Result<(), (sql_error::SqliteFail, Option<String>)>, rusqlite::Error>
where
    M: AttestEnvelopable,
{
    let data = data.inner();
    let mut stmt = tx.prepare_cached(SQL_INSERT_ENVELOPE)?;
    let time = attest_util::now();
    let genesis = data.get_genesis_hash();
    let prev_msg = data
        .header()
        .ancestors()
        .map(|m| m.prev_msg())
        .unwrap_or_else(CanonicalEnvelopeHash::genesis);
    trace!(?genesis, ?data, "attempt to insert envelope");
    let hash = data.clone().canonicalized_hash();
    match stmt.insert(rusqlite::named_params! {
                ":body": data,
                ":hash": hash,
                ":key": PK(data.header().key()),
                ":genesis": genesis,
                ":prev_msg": prev_msg,
                ":received_time": time,
                ":sent_time": data.header().sent_time_ms(),
                ":height": data.header().height(),
                ":nonce": data.header().unsigned().signature().expect("Authenticated Envelope Must Have")[0..32].to_hex()
        }) {
            Ok(_rowid) => {

                tracing::trace!(?hash, envelope=?data, "Successfully Inserted");
                tracing::info!(?hash, "Successfully Inserted");
                Ok(Ok(()))
            },
            Err(e) => match e {
                rusqlite::Error::SqliteFailure(err, msg) => match err {
                    ffi::Error {
                        code: ErrorCode::ConstraintViolation,
                        extended_code: SQLITE_CONSTRAINT_UNIQUE,
                    } => {
                        debug!(?hash, "Insert failed due to Uniqueness Constraint");
                        Ok(Err((sql_error
                    ::SqliteFail::SqliteConstraintUnique, msg)))
                    },
                    ffi::Error {
                        code: ErrorCode::ConstraintViolation,
                        extended_code: SQLITE_CONSTRAINT_NOTNULL,
                    } => {
                        debug!(?hash, "Insert failed due to Not-Null Constraint");
                        Ok(Err((sql_error
                    ::SqliteFail::SqliteConstraintNotNull, msg)))
                    },
                    ffi::Error {
                        code: ErrorCode::ConstraintViolation,
                        extended_code: SQLITE_CONSTRAINT_CHECK,
                    } => {
                        debug!(?hash, "Insert failed due to Check Constraint");
                        Ok(Err((sql_error
                    ::SqliteFail::SqliteConstraintCheck, msg)))
                    },
                    other_err => {
                        debug!(?other_err, "SQL: {}", stmt.expanded_sql().unwrap_or_default());
                        Err(rusqlite::Error::SqliteFailure(err, msg))
                    }
                },
                err =>{
                    debug!(?err, "SQL: {}", stmt.expanded_sql().unwrap_or_default());
                    Err(err)
                }
            },
        }
}
