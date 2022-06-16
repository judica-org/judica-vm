use rand::Rng;
use ruma_serde::CanonicalJsonValue;
use rusqlite::types::{FromSql, FromSqlError};
use rusqlite::ToSql;
use sapio_bitcoin::hashes::hex::ToHex;
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
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Display;
use std::str::FromStr;
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
pub struct PrecomittedNonce(pub SecretKey);

impl PrecomittedNonce {
    pub fn new<C: Signing>(secp: &Secp256k1<C>) -> Self {
        PrecomittedNonce(secp.generate_keypair(&mut rand::thread_rng()).0)
    }
    pub fn as_ptr(&self) -> *const c_void {
        assert!(self.0.len() == 32);
        self.0.as_c_ptr() as *const c_void
    }
    pub fn as_param(&self) -> SchnorrSigExtraParams {
        SchnorrSigExtraParams::new(Some(custom_nonce), self.as_ptr())
    }
    pub fn get_public<C: Signing>(&self, secp: &Secp256k1<C>) -> PrecomittedPublicNonce {
        PrecomittedPublicNonce(self.0.public_key(secp).x_only_public_key().0)
    }
}

impl ToSql for PrecomittedNonce {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.0.secret_bytes().to_hex().into())
    }
}
impl FromSql for PrecomittedNonce {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        SecretKey::from_str(value.as_str()?)
            .map(PrecomittedNonce)
            .map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Copy)]
pub struct PrecomittedPublicNonce(pub XOnlyPublicKey);
pub fn sign_with_precomitted_nonce<C: Signing>(
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
        let sig = sign_with_precomitted_nonce(&secp, &msg, &keypair, nonce);
        assert_eq!(
            sig.as_ref()[0..32],
            nonce.get_public(&secp).0.serialize()[..]
        )
    }
}
