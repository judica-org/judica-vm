use super::handle_type;

use super::MsgDBHandle;
use attest_messages::checkpoints::BitcoinCheckPoints;
use attest_messages::Ancestors;
use attest_messages::Authenticated;
use attest_messages::Envelope;
use attest_messages::Header;
use attest_messages::SigningError;
use attest_messages::Unsigned;
use ruma_serde::CanonicalJsonValue;
use sapio_bitcoin::{
    secp256k1::{Secp256k1, Signing},
    KeyPair, XOnlyPublicKey,
};

pub enum TipControl {
    GroupsOnly,
    NoTips,
    AllTips,
}
use tracing::debug;
impl<'a, T> MsgDBHandle<'a, T>
where
    T: handle_type::Get + handle_type::Insert,
{
    /// given an arbitrary inner message, generates an envelope and signs it.
    ///
    /// Calling multiple times with a given nonce would result in nonce reuse.
    pub fn wrap_message_in_envelope_for_user_by_key<C: Signing>(
        &self,
        msg: CanonicalJsonValue,
        keypair: &KeyPair,
        secp: &Secp256k1<C>,
        bitcoin_tipcache: Option<BitcoinCheckPoints>,
        dangerous_bypass_tip: Option<Envelope>,
        tip_groups: TipControl,
    ) -> Result<Result<Envelope, SigningError>, rusqlite::Error> {
        let key: XOnlyPublicKey = keypair.x_only_public_key().0;
        debug!(key=%key, "Creating new Envelope");
        // Side effect free...
        let mut tips = match tip_groups {
            TipControl::GroupsOnly => {
                self.get_all_chain_commit_group_members_tips_for_chain(key)?
            }
            TipControl::AllTips => self.get_tips_for_all_users::<Authenticated<Envelope>>()?,
            TipControl::NoTips => vec![],
        };
        if let Some(p) = tips.iter().position(|x| x.header().key() == key) {
            tips.swap_remove(p);
        }
        debug!(?tips, "Tip Envelopes");

        let tips = tips
            .iter()
            .map(|tip| {
                let h = tip.canonicalized_hash_ref();
                Some((tip.header().key(), tip.header().height(), h))
            })
            .flatten()
            .collect();
        debug!(?tips, "Extracted Tip Hashes");
        let my_tip = if let Some(envelope) = dangerous_bypass_tip {
            envelope
        } else {
            self.get_tip_for_user_by_key(key)?.inner()
        };
        let sent_time_ms = attest_util::now();
        let secret = self.get_secret_for_public_nonce(my_tip.header().next_nonce())?;
        // Has side effects!
        let next_nonce = self.generate_fresh_nonce_for_user_by_key(secp, key)?;
        let mut msg = Envelope::new(
            Header::new(
                key,
                next_nonce,
                Some(Ancestors::new(
                    my_tip.canonicalized_hash_ref(),
                    my_tip.get_genesis_hash(),
                )),
                tips,
                my_tip.header().height() + 1,
                sent_time_ms,
                Unsigned::new(Default::default()),
                bitcoin_tipcache.unwrap_or_default(),
            ),
            msg,
        );
        Ok(msg.sign_with(keypair, secp, secret).map(move |_| msg))
    }
}
