use std::sync::Arc;

use bitcoincore_rpc_async::Client;
use event_log::{
    connection::EventLog,
    db_handle::accessors::{occurrence::OccurrenceID, occurrence_group::OccurrenceGroupID},
};
use sapio_bitcoin::Network;

use crate::app::CompilerModule;

pub struct GlobalsInner {
    pub module_repo_id: OccurrenceGroupID,
    pub module_tag: String,
    pub evlog: EventLog,
    pub compiler_module: CompilerModule,
    pub bitcoin_rpc: Arc<Client>,
    pub bitcoin_network: Network,
}

pub type Globals = Arc<GlobalsInner>;
