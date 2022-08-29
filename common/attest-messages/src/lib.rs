use self::checkpoints::BitcoinCheckPoints;
use crate::nonce::{PrecomittedNonce, PrecomittedPublicNonce};

use ruma_serde::CanonicalJsonValue;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::hashes::{sha256, Hash};
use sapio_bitcoin::secp256k1::ThirtyTwoByteHash;
use sapio_bitcoin::secp256k1::{Message as SchnorrMessage, Secp256k1};
use sapio_bitcoin::secp256k1::{Signing, Verification};
use sapio_bitcoin::util::key::KeyPair;
use sapio_bitcoin::XOnlyPublicKey;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt::Display;
pub mod authenticated;
pub mod nonce;
mod util;
pub use authenticated::*;
pub mod checkpoints;
#[cfg(feature = "rusqlite")]
pub mod sql_impl;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Unsigned {
    pub signature: Option<sapio_bitcoin::secp256k1::schnorr::Signature>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Ancestors {
    pub prev_msg: CanonicalEnvelopeHash,
    pub genesis: CanonicalEnvelopeHash,
}
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct Header {
    pub key: sapio_bitcoin::secp256k1::XOnlyPublicKey,
    pub next_nonce: PrecomittedPublicNonce,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub ancestors: Option<Ancestors>,
    // tips can be out of ancestors as we may wish to still show things we came
    // after.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub tips: Vec<(XOnlyPublicKey, i64, CanonicalEnvelopeHash)>,
    pub height: i64,
    pub sent_time_ms: i64,
    pub unsigned: Unsigned,
    pub checkpoints: BitcoinCheckPoints,
}
impl std::fmt::Debug for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(self).unwrap())
    }
}
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Envelope {
    pub header: Header,
    pub msg: CanonicalJsonValue,
}

impl std::fmt::Debug for Envelope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(self).unwrap())
    }
}

#[derive(Debug)]
pub enum AuthenticationError {
    SerializerError(serde_json::Error),
    NoSignature,
    ValidationError(sapio_bitcoin::secp256k1::Error),
    HashingError,
    MissingAncestors,
    NoAncestorsForGenesis,
}
#[derive(Debug)]
pub enum SigningError {
    SerializerError(serde_json::Error),
    HashingError,
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

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, PartialOrd, Ord, Copy, Hash)]
pub struct CanonicalEnvelopeHash(sha256::Hash);
impl CanonicalEnvelopeHash {
    pub fn genesis() -> CanonicalEnvelopeHash {
        CanonicalEnvelopeHash(sha256::Hash::from_inner([0u8; 32]))
    }
    pub fn is_genesis(&self) -> bool {
        *self == Self::genesis()
    }
}
impl std::fmt::Debug for CanonicalEnvelopeHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(self).unwrap())
    }
}
impl ToHex for CanonicalEnvelopeHash {
    fn to_hex(&self) -> String {
        self.0.to_hex()
    }
}

pub struct SignatureDigest(SchnorrMessage);

impl Envelope {
    pub fn get_genesis_hash(&self) -> CanonicalEnvelopeHash {
        match self.header.ancestors {
            Some(ref a) => a.genesis,
            None => self.canonicalized_hash_ref(),
        }
    }
    /// Returns the nonce used in this [`Envelope`].
    pub fn extract_used_nonce(&self) -> Option<PrecomittedPublicNonce> {
        XOnlyPublicKey::from_slice(&self.header.unsigned.signature?.as_ref()[..32])
            .map(PrecomittedPublicNonce)
            .ok()
    }
    /// Converts this [`Envelope`] into an [`Authenticated<Envelope>`] by
    /// checking it's signature. Returns a copy.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    ///
    /// - the signature could not be
    /// validated or if one is not present.
    /// - The height is 0 and there are ancestors
    /// - The height is > 0 and there are not ancestors
    pub fn self_authenticate<C: Verification>(
        &self,
        secp: &Secp256k1<C>,
    ) -> Result<Authenticated<Self>, AuthenticationError> {
        if self.header.height == 0 && self.header.ancestors.is_some() {
            return Err(AuthenticationError::NoAncestorsForGenesis);
        }
        if self.header.height > 0 && self.header.ancestors.is_none() {
            return Err(AuthenticationError::MissingAncestors);
        }
        let mut redacted = self.clone();
        let sig = redacted
            .header
            .unsigned
            .signature
            .take()
            .ok_or(AuthenticationError::NoSignature)?;
        let msg = redacted
            .signature_digest()
            .ok_or(AuthenticationError::HashingError)?;
        secp.verify_schnorr(&sig, &msg.0, &self.header.key)
            .map_err(AuthenticationError::ValidationError)?;
        Ok(Authenticated(self.clone()))
    }

    /// signs an [`Envelope`] in-place with a given key.
    ///
    /// Because keypair is not guaranteed to be the correct keypair for the
    /// [`Envelope`], the envelope will need to be authenticated separately.
    ///
    /// This will clear any existing signatures!
    ///
    /// # Errors
    ///
    /// This function will return an error if hashing or serialization fails.
    pub fn sign_with<C: Signing>(
        &mut self,
        keypair: &KeyPair,
        secp: &Secp256k1<C>,
        nonce: PrecomittedNonce,
    ) -> Result<(), SigningError> {
        self.header.unsigned.signature = None;

        let msg = self
            .clone()
            .signature_digest()
            .ok_or(SigningError::HashingError)?;
        self.header.unsigned.signature =
            Some(nonce.sign_with_precomitted_nonce(secp, &msg.0, keypair));

        Ok(())
    }

    /// Creates the canonicalized_hash for the [`Envelope`].
    ///
    /// This hashes everything, including unsigned data.
    pub fn canonicalized_hash(self) -> CanonicalEnvelopeHash {
        self.canonicalized_hash_ref()
    }

    /// Creates the canonicalized_hash for the [`Envelope`].
    ///
    /// This hashes everything, including unsigned data.
    pub fn canonicalized_hash_ref(&self) -> CanonicalEnvelopeHash {
        let canonical =
            ruma_serde::to_canonical_value(self).expect("Canonicalization Must Succeed");
        CanonicalEnvelopeHash(sapio_bitcoin::hashes::sha256::Hash::hash(
            canonical.to_string().as_bytes(),
        ))
    }
    /// Helper to get the [`SchnorrMessage`] from an envelope.
    ///
    /// If Envelope has unsigned data present, must fail
    pub fn signature_digest(self) -> Option<SignatureDigest> {
        if self.header.unsigned.signature.is_some() {
            return None;
        }
        let msg_hash = self.canonicalized_hash();
        let msg = SchnorrMessage::from(W(msg_hash.0));
        Some(SignatureDigest(msg))
    }
}

struct W(sapio_bitcoin::hashes::sha256::Hash);
impl ThirtyTwoByteHash for W {
    fn into_32(self) -> [u8; 32] {
        self.0.into_inner()
    }
}
