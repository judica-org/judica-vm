use sapio_bitcoin::secp256k1::ffi::types::{c_int, c_uchar, c_void, size_t};
use sapio_bitcoin::secp256k1::ffi::{CPtr, SchnorrSigExtraParams};
use sapio_bitcoin::secp256k1::schnorr::Signature;
use sapio_bitcoin::secp256k1::Signing;
use sapio_bitcoin::secp256k1::{
    constants, ffi, rand, Message as SchnorrMessage, Secp256k1, SecretKey,
};
use sapio_bitcoin::util::key::KeyPair;
use sapio_bitcoin::XOnlyPublicKey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub unsafe extern "C" fn custom_nonce(
    nonce32: *mut c_uchar,
    _msg32: *const c_uchar,
    _msg_len: size_t,
    _key32: *const c_uchar,
    _xonly_pk32: *const c_uchar,
    _algo16: *const c_uchar,
    _algo_len: size_t,
    data: *mut c_void,
) -> c_int {
    nonce32.copy_from_nonoverlapping(data as *const c_uchar, 32);
    1
}

#[derive(Clone, Copy)]
pub struct PrecomittedNonce(pub SecretKey);

impl PrecomittedNonce {
    pub fn sign_with_precomitted_nonce<C: Signing>(
        self,
        secp: &Secp256k1<C>,
        msg: &SchnorrMessage,
        keypair: &KeyPair,
    ) -> Signature {
        let mut sig = [0u8; constants::SCHNORR_SIGNATURE_SIZE];
        assert_eq!(1, unsafe {
            ffi::secp256k1_schnorrsig_sign_custom(
                *secp.ctx(),
                sig.as_mut_c_ptr(),
                msg.as_c_ptr(),
                msg.len(),
                keypair.as_ptr(),
                &self.as_param(),
            )
        });
        Signature::from_slice(&sig[..]).unwrap()
    }
}

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

#[derive(
    Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Copy, JsonSchema,
)]
pub struct PrecomittedPublicNonce(#[schemars(with = "String")] pub XOnlyPublicKey);

#[cfg(test)]
mod test {

    use super::*;
    #[test]
    fn test_r_value() {
        let secp = Secp256k1::new();
        let kp = secp.generate_keypair(&mut rand::thread_rng());
        let keypair = KeyPair::from_secret_key(&secp, &kp.0);
        let nonce = PrecomittedNonce::new(&secp);
        // this is insecure...
        let msg = SchnorrMessage::from_slice(&[42u8; 32]).unwrap();
        let sig = nonce.sign_with_precomitted_nonce(&secp, &msg, &keypair);
        assert_eq!(
            sig.as_ref()[0..32],
            nonce.get_public(&secp).0.serialize()[..]
        )
    }
}
