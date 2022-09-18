use self::checkpoints::BitcoinCheckPoints;
use crate::nonce::{PrecomittedNonce, PrecomittedPublicNonce};
use ruma_serde::CanonicalJsonValue;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::hashes::{sha256, Hash};
use sapio_bitcoin::secp256k1::schnorr::Signature;
use sapio_bitcoin::secp256k1::ThirtyTwoByteHash;
use sapio_bitcoin::secp256k1::{Message as SchnorrMessage, Secp256k1};
use sapio_bitcoin::secp256k1::{Signing, Verification};
use sapio_bitcoin::util::key::KeyPair;
use sapio_bitcoin::XOnlyPublicKey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt::Display;
pub mod authenticated;
pub mod nonce;
pub use authenticated::*;
pub mod checkpoints;
#[cfg(feature = "rusqlite")]
pub mod sql_impl;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash, JsonSchema)]
pub struct Unsigned {
    #[schemars(with = "Option<String>")]
    signature: Option<sapio_bitcoin::secp256k1::schnorr::Signature>,
}

impl Unsigned {
    pub fn new(signature: Option<sapio_bitcoin::secp256k1::schnorr::Signature>) -> Self {
        Self { signature }
    }

    pub fn signature(&self) -> Option<Signature> {
        self.signature
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash, JsonSchema)]
pub struct Ancestors {
    prev_msg: CanonicalEnvelopeHash,
    genesis: CanonicalEnvelopeHash,
}

impl Ancestors {
    pub fn new(prev_msg: CanonicalEnvelopeHash, genesis: CanonicalEnvelopeHash) -> Self {
        Self { prev_msg, genesis }
    }

    pub fn prev_msg(&self) -> CanonicalEnvelopeHash {
        self.prev_msg
    }

    pub fn genesis(&self) -> CanonicalEnvelopeHash {
        self.genesis
    }
}
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Hash, JsonSchema)]
pub struct Header {
    #[schemars(with = "String")]
    key: sapio_bitcoin::secp256k1::XOnlyPublicKey,
    next_nonce: PrecomittedPublicNonce,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    ancestors: Option<Ancestors>,
    // tips can be out of ancestors as we may wish to still show things we came
    // after.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    #[schemars(with = "Vec<(String, i64, CanonicalEnvelopeHash)>")]
    tips: Vec<(XOnlyPublicKey, i64, CanonicalEnvelopeHash)>,
    height: i64,
    sent_time_ms: i64,
    unsigned: Unsigned,
    checkpoints: BitcoinCheckPoints,
}

impl Header {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        key: sapio_bitcoin::secp256k1::XOnlyPublicKey,
        next_nonce: PrecomittedPublicNonce,
        ancestors: Option<Ancestors>,
        tips: Vec<(XOnlyPublicKey, i64, CanonicalEnvelopeHash)>,
        height: i64,
        sent_time_ms: i64,
        unsigned: Unsigned,
        checkpoints: BitcoinCheckPoints,
    ) -> Self {
        Self {
            key,
            next_nonce,
            ancestors,
            tips,
            height,
            sent_time_ms,
            unsigned,
            checkpoints,
        }
    }

    pub fn checkpoints(&self) -> &BitcoinCheckPoints {
        &self.checkpoints
    }

    pub fn unsigned(&self) -> &Unsigned {
        &self.unsigned
    }

    pub fn sent_time_ms(&self) -> i64 {
        self.sent_time_ms
    }

    pub fn height(&self) -> i64 {
        self.height
    }

    pub fn tips(&self) -> &[(XOnlyPublicKey, i64, CanonicalEnvelopeHash)] {
        self.tips.as_ref()
    }

    pub fn ancestors(&self) -> Option<&Ancestors> {
        self.ancestors.as_ref()
    }

    pub fn next_nonce(&self) -> PrecomittedPublicNonce {
        self.next_nonce
    }

    pub fn key(&self) -> XOnlyPublicKey {
        self.key
    }
}
impl std::fmt::Debug for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(self).unwrap())
    }
}
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema)]
#[serde(try_from = "from_wrap::GenericErasedEnvelope")]
#[serde(bound = "T: for<'a> Deserialize<'a> + Serialize + Clone")]
pub struct GenericEnvelope<T = WrappedJson>
where
    T: AttestEnvelopable,
{
    header: Header,
    msg: T,
    #[serde(skip)]
    cache: Option<CanonicalEnvelopeHash>,
}

pub trait AttestEnvelopable
where
    Self: JsonSchema
        + for<'a> Deserialize<'a>
        + Clone
        + Serialize
        + AsRef<Self::Ref>
        + std::fmt::Debug,
    Self::Ref: ToOwned,
{
    type Ref;
    fn as_canonical(&self) -> Result<CanonicalJsonValue, serde_json::Error>;
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, JsonSchema, Debug)]
#[serde(transparent)]
pub struct WrappedJson(#[schemars(with = "Value")] CanonicalJsonValue);
impl From<CanonicalJsonValue> for WrappedJson {
    fn from(e: CanonicalJsonValue) -> Self {
        WrappedJson(e)
    }
}

impl AsRef<CanonicalJsonValue> for WrappedJson {
    fn as_ref(&self) -> &CanonicalJsonValue {
        &self.0
    }
}
impl AttestEnvelopable for WrappedJson {
    fn as_canonical(&self) -> Result<CanonicalJsonValue, serde_json::Error> {
        Ok(self.0.clone())
    }
    type Ref = CanonicalJsonValue;
}

pub type Envelope = GenericEnvelope<WrappedJson>;
impl AsRef<Envelope> for Envelope {
    fn as_ref(&self) -> &Envelope {
        self
    }
}

mod from_wrap {
    use std::convert::{TryFrom, TryInto};

    use serde::{Deserialize, Serialize};
    use serde_json::Value;

    use crate::AttestEnvelopable;

    #[derive(Deserialize)]
    pub struct GenericErasedEnvelope {
        header: super::Header,
        msg: super::CanonicalJsonValue,
    }
    impl<T> TryFrom<GenericErasedEnvelope> for super::GenericEnvelope<T>
    where
        T: AttestEnvelopable + for<'a> Deserialize<'a> + Serialize + Clone,
    {
        type Error = serde_json::Error;
        fn try_from(e: GenericErasedEnvelope) -> Result<Self, Self::Error> {
            let v: T = serde_json::from_value(e.msg.into())?;
            Ok(super::GenericEnvelope::new(e.header, v))
        }
    }
}

impl<T> GenericEnvelope<T>
where
    Self: Clone,
    T: Clone,
    T: for<'a> Deserialize<'a> + Serialize,
    T: AttestEnvelopable,
{
    pub fn new<It: Into<T>>(header: Header, msg: It) -> Self {
        let mut s = Self {
            header,
            msg: msg.into(),
            cache: None,
        };
        s.cache = Some(s.canonicalized_hash_ref());
        s
    }
}

impl<T> std::fmt::Debug for GenericEnvelope<T>
where
    T: AttestEnvelopable,
{
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

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, PartialOrd, Ord, Copy, Hash, JsonSchema)]
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
impl From<SignatureDigest> for SchnorrMessage {
    fn from(m: SignatureDigest) -> Self {
        m.0
    }
}

impl<T> GenericEnvelope<T>
where
    Self: Clone,
    T: for<'a> Deserialize<'a> + Serialize + Clone,
    T: AttestEnvelopable,
{
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
    /// Returns the sig used in this [`Envelope`].
    pub fn extract_sig_s(&self) -> Option<[u8; 32]> {
        <[u8; 32]>::try_from(&self.header.unsigned.signature?.as_ref()[32..64]).ok()
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
        redacted.cache = None;
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
        self.cache = None;

        let msg = self
            .clone()
            .signature_digest()
            .ok_or(SigningError::HashingError)?;
        self.header.unsigned.signature =
            Some(nonce.sign_with_precomitted_nonce(secp, &msg.0, keypair));
        self.cache = Some(self.compute_hash());

        Ok(())
    }

    fn compute_hash(&self) -> CanonicalEnvelopeHash {
        let canonical =
            ruma_serde::to_canonical_value(self).expect("Canonicalization Must Succeed");
        CanonicalEnvelopeHash(sapio_bitcoin::hashes::sha256::Hash::hash(
            canonical.to_string().as_bytes(),
        ))
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
        if let Some(h) = self.cache {
            h
        } else {
            self.compute_hash()
        }
    }
    /// Helper to get the [`SchnorrMessage`] from an envelope.
    ///
    /// If Envelope has unsigned data present, must fail
    pub fn signature_digest(&self) -> Option<SignatureDigest> {
        if self.header.unsigned.signature.is_some() {
            return None;
        }
        let msg_hash = self.canonicalized_hash_ref();
        let msg = SchnorrMessage::from(W(msg_hash.0));
        Some(SignatureDigest(msg))
    }
    /// Helper to get the [`SchnorrMessage`] from an envelope.
    ///
    /// If Envelope has unsigned data present, clear
    pub fn signature_digest_mut(&mut self) -> SignatureDigest {
        let sig = self.header.unsigned.signature.take();
        let cache = self.cache.take();
        let msg_hash = self.canonicalized_hash_ref();
        self.cache = cache;
        self.header.unsigned.signature = sig;
        let msg = SchnorrMessage::from(W(msg_hash.0));
        SignatureDigest(msg)
    }

    pub fn msg(&self) -> &T::Ref {
        self.msg.as_ref()
    }

    pub fn header(&self) -> &Header {
        &self.header
    }
}

struct W(sapio_bitcoin::hashes::sha256::Hash);
impl ThirtyTwoByteHash for W {
    fn into_32(self) -> [u8; 32] {
        self.0.into_inner()
    }
}
