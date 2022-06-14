use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Display;

use rand::Rng;
use ruma_serde::CanonicalJsonValue;
use sapio_bitcoin::hashes::{sha256, Hash, HashEngine, Hmac};
use sapio_bitcoin::secp256k1::ffi::types::{c_char, c_int, c_uchar, c_void, size_t};
use sapio_bitcoin::secp256k1::ffi::{CPtr, SchnorrSigExtraParams};
use sapio_bitcoin::secp256k1::schnorr::Signature;
use sapio_bitcoin::secp256k1::secp256k1_sys::Signature as InnerSig;
use sapio_bitcoin::secp256k1::{
    constants, ffi, rand, Message as SchnorrMessage, Secp256k1, SecretKey,
};
use sapio_bitcoin::secp256k1::{Signing, Verification};
use sapio_bitcoin::util::key::KeyPair;
use sapio_bitcoin::XOnlyPublicKey;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InnerMessage {
    Ping(String),
    Data(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Envelope {
    pub key: sapio_bitcoin::secp256k1::XOnlyPublicKey,
    pub channel: String,
    pub sent_time_ms: u64,
    #[serde(default)]
    pub signature: Option<sapio_bitcoin::secp256k1::schnorr::Signature>,
    pub msg: InnerMessage,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageResponse {
    Pong(String),
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
            .signature
            .take()
            .ok_or(AuthenticationError::NoSignature)?;
        let msg = redacted
            .msg_hash()
            .map_err(AuthenticationError::SerializerError)?;
        secp.verify_schnorr(&sig, &msg, &self.key)
            .map_err(AuthenticationError::ValidationError)
    }

    pub(crate) fn sign_with<C: Signing>(
        &mut self,
        keypair: &KeyPair,
        secp: &Secp256k1<C>,
    ) -> Result<(), SigningError> {
        self.signature = None;

        let msg = self
            .clone()
            .msg_hash()
            .map_err(SigningError::SerializerError)?;
        let aux_rand = rand::thread_rng().gen();
        self.signature = Some(secp.sign_schnorr_with_aux_rand(&msg, keypair, &aux_rand));

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
struct W(sapio_bitcoin::hashes::sha256::Hash);
impl ThirtyTwoByteHash for W {
    fn into_32(self) -> [u8; 32] {
        self.0.into_inner()
    }
}

pub unsafe extern "C" fn custom_nonce(
    nonce32: *mut c_uchar,
    msg32: *const c_uchar,
    msg_len: size_t,
    key32: *const c_uchar,
    xonly_pk32: *const c_uchar,
    algo16: *const c_uchar,
    algo_len: size_t,
    data: *mut c_void,
) -> c_int {
    nonce32.copy_from_nonoverlapping(data as *const c_uchar, 32);
    return 1;
}

#[derive(Clone, Copy)]
struct PrecomittedNonce(SecretKey);

impl PrecomittedNonce {
    fn new<C: Signing>(secp: &Secp256k1<C>) -> Self {
        PrecomittedNonce(secp.generate_keypair(&mut rand::thread_rng()).0)
    }
    fn as_ptr(&self) -> *const c_void {
        assert!(self.0.len() == 32);
        self.0.as_c_ptr() as *const c_void
    }
    fn as_param(&self) -> SchnorrSigExtraParams {
        SchnorrSigExtraParams::new(Some(custom_nonce), self.as_ptr())
    }
    fn get_public<C: Signing>(&self, secp: &Secp256k1<C>) -> PrecomittedPublicNonce {
        PrecomittedPublicNonce(self.0.public_key(secp).x_only_public_key().0)
    }
}

struct PrecomittedPublicNonce(XOnlyPublicKey);
fn sign_schnorr_helper<C: Signing>(
    secp: &Secp256k1<C>,
    msg: &SchnorrMessage,
    keypair: &KeyPair,
    nonce: PrecomittedNonce,
) -> Signature {
    let mut sig = [0u8; constants::SCHNORR_SIGNATURE_SIZE];
    assert_eq!(1, unsafe {
        ffi::secp256k1_schnorrsig_sign_custom(
            *secp.ctx(),
            sig.as_mut_c_ptr(),
            msg.as_c_ptr(),
            msg.len(),
            keypair.as_ptr(),
            &nonce.as_param(),
        )
    });
    Signature::from_slice(&sig[..]).unwrap()
}

#[cfg(test)]
mod test {

    use std::hash::Hash;

    use super::*;
    #[test]
    fn test_r_value() {
        let secp = Secp256k1::new();
        let kp = secp.generate_keypair(&mut rand::thread_rng());
        let keypair = KeyPair::from_secret_key(&secp, &kp.0);
        let nonce = PrecomittedNonce::new(&secp);
        // this is insecure...
        let msg = SchnorrMessage::from_slice(&[42u8; 32]).unwrap();
        let sig = sign_schnorr_helper(&secp, &msg, &keypair, nonce);
        assert_eq!(
            sig.as_ref()[0..32],
            nonce.get_public(&secp).0.serialize()[..]
        )
    }
}
