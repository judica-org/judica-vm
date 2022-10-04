use std::{collections::BTreeSet, error::Error, time::Duration};

use attest_database::{connection::MsgDB, db_handle::get::nonces::extract_sk_from_envelopes};
use bitcoin::{secp256k1::SecretKey, XOnlyPublicKey};
use event_log::{connection::EventLog, db_handle::accessors::occurrence_group::OccurrenceGroupID};
use simps::{DLogDiscovered, EK_NEW_DLOG};
use tokio::{spawn, time};
use tracing::{debug, info, trace};

use crate::{Event, OK_T};

pub async fn dlog_extractor(
    msg_db: MsgDB,
    evlog: EventLog,
    evlog_group_id: OccurrenceGroupID,
    interval: Duration,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut known: BTreeSet<XOnlyPublicKey> = Default::default();
    loop {
        time::sleep(interval).await;

        let mut reused_nonce_map = {
            let hdl = msg_db.get_handle().await;
            hdl.get_reused_nonces().map_err(|e| {
                tracing::error!(error=?e, "Failed to fetch reused nonces");
                e
            })?
        };

        // remove ones we already learned so we don't put it in the evlog more
        // than once
        for x in known.iter() {
            reused_nonce_map.remove(x);
        }

        for (k, mut v) in reused_nonce_map {
            let e1 = v
                .pop()
                .ok_or("Invariant Broken in Database, reused nonce returned fewer than two")?;
            let e2 = v
                .pop()
                .ok_or("Invariant Broken in Database, reused nonce returned fewer than two")?;
            if let Some(dlog_discovered) = extract_sk_from_envelopes(e1, e2) {
                info!(
                    ?k,
                    ?dlog_discovered,
                    "Learned DLog of an Attestation Chain, Evidence of Equivocation Acquired!"
                );
                known.insert(k);
                // break if error, since this serialization should never fail.
                let msg = serde_json::to_value(DLogDiscovered { dlog_discovered })?;
                // break if DB error
                let _eid = evlog.get_accessor().await.insert_new_occurrence_now_from(
                    evlog_group_id,
                    &Event::NewRecompileTriggeringObservation(msg, EK_NEW_DLOG.clone()),
                )?;
            } else {
                debug!(?k, "Expected to learn DLog, but it failed. Try manually inspecting the envelopes for k.");
            }
        }
    }
}
