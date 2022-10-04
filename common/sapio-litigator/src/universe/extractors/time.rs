use event_log::{connection::EventLog, db_handle::accessors::occurrence_group::OccurrenceGroupID};
use std::{error::Error, sync::Arc, time::Duration};
use tokio::sync::Notify;

use crate::events::{self};

pub async fn time_event_extractor(
    evlog: EventLog,
    evlog_group_id: OccurrenceGroupID,
    new_synthetic_event: Arc<Notify>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        let accessor = evlog.get_accessor().await;
        // no need for a tag because this is always fresh
        let o = events::TaggedEvent(
            events::Event::SyntheticPeriodicActions(attest_util::now()),
            None,
        );
        accessor.insert_new_occurrence_now_from(evlog_group_id, &o)?;
        new_synthetic_event.notify_one();
    }
}
