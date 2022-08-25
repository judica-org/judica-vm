use super::handle_type;
use super::MsgDBHandle;
use attest_messages::checkpoints::BitcoinCheckPointCache;
use attest_messages::checkpoints::BitcoinCheckPoints;
use attest_messages::Envelope;
use attest_messages::Header;
use attest_messages::SigningError;
use attest_messages::Unsigned;
use sapio_bitcoin::{
    secp256k1::{Secp256k1, Signing},
    KeyPair, XOnlyPublicKey,
};
use serde_json::Value;
impl<'a, T> MsgDBHandle<'a, T>
where
    T: handle_type::Get + handle_type::Insert,
{
    /// given an arbitrary inner message, generates an envelope and signs it.
    ///
    /// Calling multiple times with a given nonce would result in nonce reuse.
    pub fn wrap_message_in_envelope_for_user_by_key<C: Signing>(
        &self,
        msg: Value,
        keypair: &KeyPair,
        secp: &Secp256k1<C>,
        bitcoin_tipcache: Option<BitcoinCheckPoints>,
        dangerous_bypass_tip: Option<Envelope>,
    ) -> Result<Result<Envelope, SigningError>, rusqlite::Error> {
        let key: XOnlyPublicKey = keypair.x_only_public_key().0;
        // Side effect free...
        let tips = self.get_tips_for_all_users()?;
        let my_tip = if let Some(envelope) = dangerous_bypass_tip {
            envelope
        } else {
            self.get_tip_for_user_by_key(key)?
        };
        let sent_time_ms = attest_util::now();
        let secret = self.get_secret_for_public_nonce(my_tip.header.next_nonce)?;
        // Has side effects!
        let next_nonce = self.generate_fresh_nonce_for_user_by_key(secp, key)?;
        let mut msg = Envelope {
            header: Header {
                height: my_tip.header.height + 1,
                genesis: if my_tip.header.genesis.is_genesis() {
                    my_tip.canonicalized_hash_ref().unwrap()
                } else {
                    my_tip.header.genesis
                },
                prev_msg: my_tip.canonicalized_hash_ref().unwrap(),
                tips: tips
                    .iter()
                    .map(|tip| {
                        let h = tip.clone().canonicalized_hash()?;
                        Some((tip.header.key, tip.header.height, h))
                    })
                    .flatten()
                    .collect(),
                next_nonce,
                key,
                sent_time_ms,
                unsigned: Unsigned {
                    signature: Default::default(),
                },
                checkpoints: bitcoin_tipcache.unwrap_or_default(),
            },
            msg,
        };
        Ok(msg.sign_with(keypair, secp, secret).map(move |_| msg))
    }
}
