use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Display;

use rand::Rng;
use ruma_serde::CanonicalJsonValue;
use rusqlite::types::FromSql;
use rusqlite::ToSql;
use sapio_bitcoin::hashes::{sha256, Hash, HashEngine, Hmac};
use sapio_bitcoin::secp256k1::ffi::types::{c_char, c_int, c_uchar, c_void, size_t};
use sapio_bitcoin::secp256k1::ffi::{CPtr, SchnorrSigExtraParams};
use sapio_bitcoin::secp256k1::schnorr::Signature;
use sapio_bitcoin::secp256k1::{
    constants, ffi, rand, Message as SchnorrMessage, Secp256k1, SecretKey,
};
use sapio_bitcoin::secp256k1::{Signing, Verification};
use sapio_bitcoin::util::key::KeyPair;
use sapio_bitcoin::XOnlyPublicKey;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InnerMessage {
    Data(String),
    Ping(u64),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Unsigned {
    pub signature: Option<sapio_bitcoin::secp256k1::schnorr::Signature>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    pub key: sapio_bitcoin::secp256k1::XOnlyPublicKey,
    pub next_nonce: PrecomittedPublicNonce,
    pub sent_time_ms: u64,
    pub unsigned: Unsigned,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Envelope {
    pub header: Header,
    pub msg: InnerMessage,
}
impl ToSql for Envelope {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let s = serde_json::to_string(self)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(s.into())
    }
}
impl FromSql for Envelope {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        serde_json::from_str(s).map_err(|e| rusqlite::types::FromSqlError::Other(e.into()))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageResponse {
    Pong(u64, u64),
    None,
}

#[derive(Debug)]
pub enum AuthenticationError {
    SerializerError(serde_json::Error),
    NoSignature,
    ValidationError(sapio_bitcoin::secp256k1::Error),
}
#[derive(Debug)]
pub enum SigningError {
    SerializerError(serde_json::Error),
}

impl Display for SigningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Display for AuthenticationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Error for SigningError {}
impl Error for AuthenticationError {}
impl Envelope {
    pub fn self_authenticate<C: Verification>(
        &self,
        secp: &Secp256k1<C>,
    ) -> Result<(), AuthenticationError> {
        let mut redacted = self.clone();
        let sig = redacted
            .header
            .unsigned
            .signature
            .take()
            .ok_or(AuthenticationError::NoSignature)?;
        let msg = redacted
            .msg_hash()
            .map_err(AuthenticationError::SerializerError)?;
        secp.verify_schnorr(&sig, &msg, &self.header.key)
            .map_err(AuthenticationError::ValidationError)
    }

    pub(crate) fn sign_with<C: Signing>(
        &mut self,
        keypair: &KeyPair,
        secp: &Secp256k1<C>,
        nonce: PrecomittedNonce,
    ) -> Result<(), SigningError> {
        self.header.unsigned.signature = None;

        let msg = self
            .clone()
            .msg_hash()
            .map_err(SigningError::SerializerError)?;
        self.header.unsigned.signature =
            Some(sign_with_precomitted_nonce(secp, &msg, keypair, nonce));

        Ok(())
    }
    fn msg_hash(self) -> Result<SchnorrMessage, serde_json::Error> {
        let msg_str = serde_json::to_value(self)
            .and_then(|reserialized| serde_json::from_value(reserialized))
            .and_then(|canonicalized: BTreeMap<String, CanonicalJsonValue>| {
                serde_json::to_string(&canonicalized)
            })?;
        let msg_hash = sapio_bitcoin::hashes::sha256::Hash::hash(msg_str.as_bytes());
        let msg = SchnorrMessage::from(W(msg_hash));
        Ok(msg)
    }
}

use sapio_bitcoin::secp256k1::ThirtyTwoByteHash;

use super::nonce::{sign_with_precomitted_nonce, PrecomittedNonce, PrecomittedPublicNonce};
struct W(sapio_bitcoin::hashes::sha256::Hash);
impl ThirtyTwoByteHash for W {
    fn into_32(self) -> [u8; 32] {
        self.0.into_inner()
    }
}
