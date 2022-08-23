use super::sql_serializers::{self};
use rusqlite::Connection;
use tokio::sync::MutexGuard;

mod get;
mod insert;
mod setup;
mod create;
pub struct MsgDBHandle<'a>(pub MutexGuard<'a, Connection>);

pub enum ConsistentMessages {
    AllMessagesNotReady,
}
