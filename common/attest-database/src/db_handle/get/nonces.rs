use super::super::handle_type;
use super::super::MsgDBHandle;
use attest_messages::nonce::PrecomittedNonce;
use attest_messages::nonce::PrecomittedPublicNonce;
use attest_messages::Authenticated;
use attest_messages::Envelope;
use fallible_iterator::FallibleIterator;
use num_bigint::BigInt;
use num_bigint::Sign;
use num_integer::Integer;
use sapio_bitcoin::hashes::Hash;
use sapio_bitcoin::hashes::sha256;
use sapio_bitcoin::hashes::HashEngine;
use sapio_bitcoin::secp256k1::Message;
use sapio_bitcoin::secp256k1::SecretKey;
use sapio_bitcoin::XOnlyPublicKey;
use std::collections::HashMap;

impl<'a, T> MsgDBHandle<'a, T>
where
    T: handle_type::Get,
{
    /// Returns the secret nonce for a given public nonce
    pub fn get_secret_for_public_nonce(
        &self,
        nonce: PrecomittedPublicNonce,
    ) -> Result<PrecomittedNonce, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached("SELECT (private_key) FROM message_nonces where public_key = ?")?;
        stmt.query_row([nonce], |r| r.get::<_, PrecomittedNonce>(0))
    }

    /// finds a reused nonce
    pub fn get_reused_nonces(
        &self,
    ) -> Result<HashMap<XOnlyPublicKey, Vec<Authenticated<Envelope>>>, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare_cached(include_str!("../sql/get/reused_nonces.sql"))?;
        let rows = stmt.query([])?;
        let vs = rows.map(|r| r.get::<_, Authenticated<Envelope>>(0)).fold(
            HashMap::new(),
            |mut acc, v| {
                acc.entry(v.header().key()).or_insert(vec![]).push(v);
                Ok(acc)
            },
        )?;

        Ok(vs)
    }
}

pub fn extract_sk_from_envelopes(
    e1: Authenticated<Envelope>,
    e2: Authenticated<Envelope>,
) -> Option<SecretKey> {
    let mut e1 = e1.inner();
    let mut e2 = e2.inner();
    let nonce = e1.extract_used_nonce()?;
    let key = e1.header().key();
    if key != e2.header().key() {
        return None;
    }
    if nonce != e2.extract_used_nonce()? {
        return None;
    }
    let m1 = e1.signature_digest_mut();
    let m2 = e2.signature_digest_mut();
    let s1 = e1.extract_sig_s()?;
    let s2 = e2.extract_sig_s()?;
    extract_sk(key, m1, m2, &nonce.0.serialize(), &s1, &s2)
}

pub fn extract_sk<M1, M2>(
    key: XOnlyPublicKey,
    m1: M1,
    m2: M2,
    nonce: &[u8; 32],
    s1: &[u8; 32],
    s2: &[u8; 32],
) -> Option<SecretKey>
where
    Message: From<M1> + From<M2>,
{
    // H(tag || tag || R || P || m)
    let mut engine = get_signature_tagged_hash();

    engine.input(&nonce[..]);
    engine.input(&key.serialize()[..]);
    let mut engine2 = engine.clone();
    engine.input(Message::from(m1).as_ref());
    engine2.input(Message::from(m2).as_ref());

    let d1 = sha256::Hash::from_engine(engine);
    let d2 = sha256::Hash::from_engine(engine2);

    //    s1 - s2 / d1 - d2 = p

    let s1 = BigInt::from_bytes_be(Sign::Plus, &s1[..]);
    let s2 = BigInt::from_bytes_be(Sign::Plus, &s2[..]);

    let d1 = BigInt::from_bytes_be(Sign::Plus, &d1[..]);
    let d2 = BigInt::from_bytes_be(Sign::Plus, &d2[..]);
    let divisor = d1 - d2;
    let field = BigInt::from_bytes_be(
        Sign::Plus,
        &[
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 254, 186,
            174, 220, 230, 175, 72, 160, 59, 191, 210, 94, 140, 208, 54, 65, 65,
        ][..],
    );

    let res = divisor.extended_gcd(&field);
    #[cfg(test)]
    {
        let res = res.clone();
        let field = field.clone();
        let divisor = divisor.clone();
        assert_eq!(&res.gcd, &1u32.into());
        assert_eq!((res.x * divisor).mod_floor(&field), 1u32.into());
    }

    let inv = res.x.mod_floor(&field);
    let result = (inv * (s1 - s2)).mod_floor(&field);

    let (s, mut sig_bytes) = result.to_bytes_le();
    assert!(s == Sign::Plus);
    while sig_bytes.len() < 32 {
        sig_bytes.push(0);
    }
    sig_bytes.reverse();
    SecretKey::from_slice(&sig_bytes[..]).ok()
}

pub(crate) fn get_signature_tagged_hash() -> sha256::HashEngine {
    let tag = sha256::Hash::hash("BIP0340/challenge".as_bytes());
    let mut engine = sha256::Hash::engine();
    engine.input(&tag.as_inner()[..]);
    engine.input(&tag.as_inner()[..]);
    engine
}
