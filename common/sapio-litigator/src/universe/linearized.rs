use std::{error::Error, sync::Arc, time::Duration};

use event_log::{
    connection::EventLog,
    db_handle::accessors::{
        occurrence::{OccurrenceID, ToOccurrence},
        occurrence_group::OccurrenceGroupID,
    },
};
use tokio::sync::{mpsc::Sender, Notify};
use tracing::{trace, warn};

use crate::{
    events::Event,
    events::{self, TaggedEvent},
};

// About 25 steps, 1.5 mins, max wait 30s
const MAX_WAIT_TO_CHECK_LOG: Duration = Duration::from_secs(30);
const LOG_CHECK_BACKING: f64 = 1.5;
const LOG_CHECK_START: Duration = Duration::from_millis(1);
pub async fn event_log_processor(
    evlog: EventLog,
    evlog_group_id: OccurrenceGroupID,
    tx: Sender<events::Event>,
    new_events_in_evlog: Arc<Notify>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut last = OccurrenceID::before_first_row();
    let mut time_to_wait = LOG_CHECK_START;
    let mut wait_for_new_synth = new_events_in_evlog.notified();
    loop {
        // Wait till we get notified of new events being in the event log
        // OR until our timer goes off
        // record our waking reason to log warnings
        trace!(?time_to_wait, "Event Log Processor Sleeping");
        let woke_by_notif = tokio::select! {
            _ = wait_for_new_synth => {true}
            _ = tokio::time::sleep(time_to_wait) => {false}
        };

        trace!(woke_by_notif, "Waking Up to Process Events");

        wait_for_new_synth = {
            let (to_process, has_new) = {
                trace!("waiting to get evlog");
                let accessor = evlog.get_accessor().await;
                trace!("evlog aquired");
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
                (time_to_wait.mul_f64(LOG_CHECK_BACKING)).min(MAX_WAIT_TO_CHECK_LOG)
            } else {
                LOG_CHECK_START
            };
            // iterate over all the Ocurrences in the DB.
            // If any Occurrence can't be processed as an Event, return.
            for (occurrence_id, occurrence) in to_process {
                let TaggedEvent(ev, _) = TaggedEvent::from_occurrence(occurrence)?;
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
