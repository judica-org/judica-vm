use std::{error::Error, sync::Arc, time::Duration};

use event_log::{
    connection::EventLog,
    db_handle::accessors::{
        occurrence::{OccurrenceID, ToOccurrence},
        occurrence_group::OccurrenceGroupID,
    },
};
use tokio::sync::{mpsc::Sender, Notify};
use tracing::warn;

use crate::Event;

// About 25 steps, 1.5 mins, max wait 30s
const MAX_WAIT_TO_CHECK_LOG: f64 = 30.0;
const LOG_CHECK_BACKING: f64 = 1.5;
const LOG_CHECK_START: f64 = 0.001;
pub async fn event_log_processor(
    evlog: EventLog,
    evlog_group_id: OccurrenceGroupID,
    tx: Sender<Event>,
    new_events_in_evlog: Arc<Notify>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut last = OccurrenceID::before_first_row();
    let mut time_to_wait = LOG_CHECK_START;
    let mut wait_for_new_synth = new_events_in_evlog.notified();
    loop {
        // Wait till we get notified of new events being in the event log
        // OR until our timer goes off
        // record our waking reason to log warnings
        let woke_by_notif = tokio::select! {
            _ = wait_for_new_synth => {true}
            _ = tokio::time::sleep(Duration::from_secs_f64(time_to_wait)) => {false}
        };
        wait_for_new_synth = {
            let (to_process, has_new) = {
                let accessor = evlog.get_accessor().await;
                let to_process =
                    accessor.get_occurrences_for_group_after_id(evlog_group_id, last)?;
                // capture with lock to be more certain to catch next notification
                (to_process, new_events_in_evlog.notified())
            };
            //  Back out of timeout if nothing in to_process
            time_to_wait = if to_process.is_empty() {
                if woke_by_notif {
                    warn!("Notified of new events but none were found");
                }
                (time_to_wait * LOG_CHECK_BACKING).min(MAX_WAIT_TO_CHECK_LOG)
            } else {
                LOG_CHECK_START
            };
            // iterate over all the Ocurrences in the DB.
            // If any Occurrence can't be processed as an Event, return.
            for (occurrence_id, occurrence) in to_process {
                let ev = Event::from_occurrence(occurrence)?;
                if tx.send(ev).await.is_err() {
                    // the reciever has been dropped
                    return Ok(());
                }
                last = occurrence_id;
            }
            has_new
        };
    }
}