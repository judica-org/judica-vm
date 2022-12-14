// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::sql_error::SqliteFail;

use super::handle_type;

use super::MsgDBHandle;
use attest_messages::checkpoints::BitcoinCheckPoints;
use attest_messages::Ancestors;
use attest_messages::AttestEnvelopable;
use attest_messages::Authenticated;
use attest_messages::Envelope;
use attest_messages::GenericEnvelope;
use attest_messages::Header;
use attest_messages::SigningError;
use attest_messages::Unsigned;
use attest_messages::WrappedJson;

use sapio_bitcoin::secp256k1::Verification;
use sapio_bitcoin::{
    secp256k1::{Secp256k1, Signing},
    KeyPair, XOnlyPublicKey,
};

#[derive(Clone, Copy)]
pub enum TipControl {
    GroupsOnly,
    NoTips,
    AllTips,
}
use tracing::debug;
use tracing::warn;
impl<T> MsgDBHandle<T>
where
    T: handle_type::Get + handle_type::Insert,
{
    /// given an arbitrary inner message, generates an envelope and signs it.
    ///
    /// Calling multiple times with a given nonce would result in nonce reuse.
    pub fn wrap_message_in_envelope_for_user_by_key<
        C: Signing,
        M: AttestEnvelopable,
        Im: Into<M>,
    >(
        &self,
        msg: Im,
        keypair: &KeyPair,
        secp: &Secp256k1<C>,
        bitcoin_tipcache: Option<BitcoinCheckPoints>,
        dangerous_bypass_tip: Option<Envelope>,
        tip_groups: TipControl,
    ) -> Result<Result<GenericEnvelope<M>, SigningError>, rusqlite::Error> {
        let key: XOnlyPublicKey = keypair.x_only_public_key().0;
        debug!(key=%key, "Creating new Envelope");
        // Side effect free...
        let mut tips = match tip_groups {
            TipControl::GroupsOnly => {
                self.get_all_chain_commit_group_members_tips_for_chain(key, true)?
            }
            TipControl::AllTips => {
                // N.B. get the WrappedJson typed tips because we don't care what their inner message type was.
                self.get_tips_for_all_users::<Authenticated<Envelope>, WrappedJson>()?
            }
            TipControl::NoTips => vec![],
        };
        if let Some(p) = tips.iter().position(|x| x.header().key() == key) {
            tips.swap_remove(p);
        }
        debug!(?tips, "Tip Envelopes");

        let tips: Vec<(XOnlyPublicKey, i64, attest_messages::CanonicalEnvelopeHash)> = tips
            .iter()
            .map(|tip| {
                let h = tip.canonicalized_hash_ref();
                (tip.header().key(), tip.header().height(), h)
            })
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
        let mut msg = GenericEnvelope::new(
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
            msg.into(),
        );
        Ok(msg.sign_with(keypair, secp, secret).map(move |_| msg))
    }

    pub fn retry_insert_authenticated_envelope_atomic<M, C, Im>(
        &mut self,
        msg: Im,
        keypair: &KeyPair,
        secp: &Secp256k1<C>,
        bitcoin_tipcache: Option<BitcoinCheckPoints>,
        tip_groups: TipControl,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
    where
        M: AttestEnvelopable,
        C: Signing + Verification,
        Im: Into<M> + Clone,
    {
        loop {
            let wrapped = self
                .wrap_message_in_envelope_for_user_by_key::<_, M, Im>(
                    msg.clone(),
                    keypair,
                    secp,
                    bitcoin_tipcache.clone(),
                    None,
                    tip_groups,
                )??
                .self_authenticate(secp)?;
            match self
                .try_insert_authenticated_envelope(wrapped, true)?
                .map_err(|(a, sqlite_error_extra)| {
                    warn!(?sqlite_error_extra, "Failed to Insert");
                    a
                }) {
                Ok(()) => break Ok(()),
                Err(SqliteFail::SqliteConstraintUnique) => {}
                Err(e) => break Err(e)?,
            }
        }
    }
}
