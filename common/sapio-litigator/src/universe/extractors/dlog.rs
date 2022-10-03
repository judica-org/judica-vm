use std::error::Error;

use attest_database::{connection::MsgDB, db_handle::get::nonces::extract_sk_from_envelopes};
use event_log::{connection::EventLog, db_handle::accessors::occurrence_group::OccurrenceGroupID};
use simps::DLogDiscovered;
use tokio::spawn;

use crate::Event;

pub async fn dlog_extractor(
    msg_db: MsgDB,
    evlog: EventLog,
    evlog_group_id: OccurrenceGroupID,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    spawn(async move {
        loop {
            let hdl = msg_db.get_handle().await;
            let reused = hdl.get_reused_nonces();
            drop(hdl);
            match reused {
                Ok(reused_nonce_map) => {
                    for (k, mut v) in reused_nonce_map {
                        match (v.pop(), v.pop()) {
                            (Some(e1), Some(e2)) => {
                                if let Some(dlog_value) = extract_sk_from_envelopes(e1, e2)
                                    .and_then(|sk| {
                                        serde_json::to_value(DLogDiscovered {
                                            dlog_discovered: sk,
                                        })
                                        .ok()
                                    })
                                {
                                    let accessor = evlog.get_accessor().await;
                                    accessor.insert_new_occurrence_now_from(
                                        evlog_group_id,
                                        &Event::NewRecompileTriggeringObservation(dlog_value),
                                    );
                                    drop(accessor);
                                }
                            }
                            _ => {
                                continue;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(error=?e, "Failed to fetch reused nonces");
                    break;
                }
            }
        }
    });
    Ok(())
}
