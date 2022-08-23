use super::MsgDBHandle;
use attest_messages::nonce::PrecomittedNonce;
use attest_messages::nonce::PrecomittedPublicNonce;
use attest_messages::Authenticated;
use attest_messages::Envelope;
use rusqlite::params;
use sapio_bitcoin::{
    hashes::hex::ToHex,
    secp256k1::{Secp256k1, Signing},
    KeyPair, XOnlyPublicKey,
};
impl<'a> MsgDBHandle<'a> {
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
        stmt.insert(rusqlite::params![key.to_hex(), pk_nonce, nonce,])?;
        Ok(pk_nonce)
    }

    /// adds a hidden service to our connection list
    /// Won't fail if already exists
    pub fn insert_hidden_service(&self, s: String, port: u16) -> Result<(), rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("INSERT OR IGNORE INTO hidden_services (service_url, port) VALUES (?,?)")?;
        stmt.insert(rusqlite::params![s, port])?;
        Ok(())
    }

    /// saves a keypair to our keyset
    pub fn save_keypair(&self, kp: KeyPair) -> Result<(), rusqlite::Error> {
        let mut stmt = self.0
                                .prepare("
                                            INSERT INTO private_keys (public_key, private_key) VALUES (?, ?)
                                            ")?;
        stmt.insert(rusqlite::params![
            kp.x_only_public_key().0.to_hex(),
            kp.secret_bytes().to_hex()
        ])?;
        Ok(())
    }

    /// creates a new user from a genesis envelope
    pub fn insert_user_by_genesis_envelope(
        &self,
        nickname: String,
        envelope: Authenticated<Envelope>,
    ) -> Result<String, rusqlite::Error> {
        let mut stmt = self
            .0
            .prepare("INSERT INTO users (nickname, key) VALUES (?, ?)")?;
        let hex_key = envelope.inner_ref().header.key.to_hex();
        stmt.insert(params![nickname, hex_key])?;
        self.try_insert_authenticated_envelope(envelope)?;
        Ok(hex_key)
    }
    /// attempts to put an authenticated envelope in the DB
    ///
    /// Will fail if the key is not registered.
    pub fn try_insert_authenticated_envelope(
        &self,
        data: Authenticated<Envelope>,
    ) -> Result<(), rusqlite::Error> {
        let data = data.inner();
        let mut stmt = self.0.prepare(include_str!("sql/insert/envelope.sql"))?;
        let time = attest_util::now();

        stmt.insert(rusqlite::named_params! {
                ":body": data,
                ":hash": data.clone()
                    .canonicalized_hash()
                    .expect("Hashing should always succeed?"),
                ":key": data.header.key.to_hex(),
                ":received_time": time
        })?;
        Ok(())
    }
}
