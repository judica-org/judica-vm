use std::sync::Arc;

use event_log::{
    connection::EventLog,
    db_handle::accessors::{occurrence::OccurrenceID, occurrence_group::OccurrenceGroupID},
};

use crate::app::CompilerModule;

pub struct GlobalsInner {
    pub module_repo_id: OccurrenceGroupID,
    pub module_tag: String,
    pub evlog: EventLog,
    pub compiler_module: CompilerModule,
}

pub type Globals = Arc<GlobalsInner>;
