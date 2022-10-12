// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{collections::BTreeSet, error::Error, time::Duration};

use attest_database::{connection::MsgDB, db_handle::get::nonces::extract_sk_from_envelopes};
use bitcoin::{hashes::hex::ToHex, XOnlyPublicKey};
use event_log::{
    connection::EventLog,
    db_handle::accessors::{occurrence::sql::Idempotent, occurrence_group::OccurrenceGroupID},
};
use simps::{DLogDiscovered, EK_NEW_DLOG};
use tokio::{task::spawn_blocking, time};
use tracing::{debug, info};

use crate::{events::Event, events::Tag, events::TaggedEvent};

pub async fn dlog_extractor(
    msg_db: MsgDB,
    evlog: EventLog,
    evlog_group_id: OccurrenceGroupID,
    interval: Duration,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let res = dlog_extractor_inner(msg_db, evlog, evlog_group_id, interval).await;
    debug!(with = ?res, "DLog Extractor Terminated");
    res
}
pub async fn dlog_extractor_inner(
    msg_db: MsgDB,
    evlog: EventLog,
    evlog_group_id: OccurrenceGroupID,
    interval: Duration,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut known: BTreeSet<XOnlyPublicKey> = Default::default();
    loop {
        time::sleep(interval).await;

        info!("DLog Extractor Waking Up");
        let mut reused_nonce_map = {
            let hdl = msg_db.get_handle_read().await;

            spawn_blocking(move || {
                hdl.get_reused_nonces().map_err(|e| {
                    tracing::error!(error=?e, "Failed to fetch reused nonces");
                    e
                })
            })
            .await??
        };

        // remove ones we already learned so we don't put it in the evlog more
        // than once
        for x in known.iter() {
            reused_nonce_map.remove(x);
        }

        for (k, mut v) in reused_nonce_map {
            info!(key=?k, "New DLog For");
            let e1 = v
                .pop()
                .ok_or("Invariant Broken in Database, reused nonce returned fewer than two")?;
            let e2 = v
                .pop()
                .ok_or("Invariant Broken in Database, reused nonce returned fewer than two")?;
            info!("Messages: {} {}", e1.msg(), e2.msg());
            if let Some(dlog_discovered) = extract_sk_from_envelopes(e1, e2) {
                info!(
                    ?k,
                    secret_key_hex = dlog_discovered.secret_bytes().to_hex(),
                    "Learned DLog of an Attestation Chain, Evidence of Equivocation Acquired!"
                );
                known.insert(k);
                // break if error, since this serialization should never fail.
                let msg = serde_json::to_value(DLogDiscovered { dlog_discovered })?;
                // break if DB error
                match evlog.get_accessor().await.insert_new_occurrence_now_from(
                    evlog_group_id,
                    &TaggedEvent(
                        Event::NewRecompileTriggeringObservation(msg, EK_NEW_DLOG.clone()),
                        Some(Tag::ScopedValue(
                            "dlog".into(),
                            dlog_discovered.secret_bytes().to_hex(),
                        )),
                    ),
                )? {
                    Ok(_) => {}
                    Err(Idempotent::AlreadyExists) => {}
                }
            } else {
                debug!(?k, "Expected to learn DLog, but it failed. Try manually inspecting the envelopes for k.");
            }
        }
    }
}
