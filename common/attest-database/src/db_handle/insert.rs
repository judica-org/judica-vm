use super::handle_type;
use super::MsgDBHandle;
use crate::sql_serializers::PK;
use crate::sql_serializers::SK;
use attest_messages::nonce::PrecomittedNonce;
use attest_messages::nonce::PrecomittedPublicNonce;
use attest_messages::Authenticated;
use attest_messages::CanonicalEnvelopeHash;
use attest_messages::Envelope;
use rusqlite::ffi;

use rusqlite::params;
use rusqlite::ErrorCode;
use sapio_bitcoin::{
    hashes::hex::ToHex,
    secp256k1::{Secp256k1, Signing},
    KeyPair, XOnlyPublicKey,
};

use std::os::raw::c_int;
use tracing::debug;
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
        let mut stmt = self.0.prepare(include_str!("sql/insert/nonce.sql"))?;
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
        let mut stmt = self
            .0
            .prepare(include_str!("sql/insert/hidden_service.sql"))?;
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
        let mut stmt = self.0
                                .prepare("
                                            INSERT INTO private_keys (public_key, private_key) VALUES (?, ?)
                                            ")?;
        stmt.insert(rusqlite::params![
            PK(kp.x_only_public_key().0),
            SK(kp.secret_key())
        ])?;
        Ok(())
    }

    /// creates a new user from a genesis envelope
    #[must_use]
    pub fn insert_user_by_genesis_envelope(
        &self,
        nickname: String,
        envelope: Authenticated<Envelope>,
    ) -> Result<Result<String, SqliteFail>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("INSERT INTO users (nickname, key) VALUES (?, ?)")?;
        let hex_key = PK(envelope.inner_ref().header.key);
        stmt.insert(params![nickname, hex_key])?;
        self.try_insert_authenticated_envelope(envelope)
            .map(|t| t.and(Ok(hex_key.0.to_hex())))
    }
    /// attempts to put an authenticated envelope in the DB
    ///
    /// Will fail if the key is not registered.
    ///
    /// Will return false if the message already existed
    #[must_use]
    pub fn try_insert_authenticated_envelope(
        &self,
        data: Authenticated<Envelope>,
    ) -> Result<Result<(), SqliteFail>, rusqlite::Error> {
        let data = data.inner();
        let mut stmt = self.0.prepare(include_str!("sql/insert/envelope.sql"))?;
        let time = attest_util::now();
        let genesis = data.get_genesis_hash();
        let prev_msg = data
            .header
            .ancestors
            .as_ref()
            .map(|m| m.prev_msg)
            .unwrap_or(CanonicalEnvelopeHash::genesis());
        debug!(?genesis, ?data);
        match stmt.insert(rusqlite::named_params! {
                ":body": data,
                ":hash": data.clone()
                    .canonicalized_hash()
                    .expect("Hashing should always succeed?"),
                ":key": PK(data.header.key),
                ":genesis": genesis,
                ":prev_msg": prev_msg,
                ":received_time": time,
                ":sent_time": data.header.sent_time_ms,
                ":height": data.header.height,
                ":nonce": data.header.unsigned.signature.expect("Authenticated Envelope Must Have")[0..32].to_hex()
        }) {
            Ok(_rowid) => Ok(Ok(())),
            Err(e) => match e {
                rusqlite::Error::SqliteFailure(err, ref _msg) => match err {
                    ffi::Error {
                        code: ErrorCode::ConstraintViolation,
                        extended_code: SQLITE_CONSTRAINT_UNIQUE,
                    } => Ok(Err(SqliteFail::SqliteConstraintUnique)),
                    ffi::Error {
                        code: ErrorCode::ConstraintViolation,
                        extended_code: SQLITE_CONSTRAINT_NOTNULL,
                    } => Ok(Err(SqliteFail::SqliteConstraintNotNull)),
                    _ => {
                        debug!("SQL: {}", stmt.expanded_sql().unwrap_or_default());
                        Err(e)
                    }
                },
                err => Err(err),
            },
        }
    }
}

/// Constant for Unique Contraint Violation
/// Yes, pattern matching works.
///```
/// use std::os::raw::c_int;
/// const X: c_int = 0;
/// struct Y {
///     val: c_int,
/// }
/// match (Y { val: 1 }) {
///     Y { val: X } => panic!("bad"),
///     Y { val: b } => println!("good"),
/// }
/// match (Y { val: 0 }) {
///     Y { val: X } => println!("good"),
///     Y { val: b } => panic!("bad"),
/// }
///```
const SQLITE_CONSTRAINT_UNIQUE: c_int = SqliteFail::SqliteConstraintUnique as c_int;
const SQLITE_CONSTRAINT_NOTNULL: c_int = SqliteFail::SqliteConstraintNotNull as c_int;
#[must_use]
#[derive(Debug)]
#[repr(C)]
pub enum SqliteFail {
    SqliteConstraintUnique = 2067,
    SqliteConstraintNotNull = 1299,
}

impl std::fmt::Display for SqliteFail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for SqliteFail {}
