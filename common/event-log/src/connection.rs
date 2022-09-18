use super::db_handle::EventLogAccessor;
use crate::db_handle::accessor_type;
use rusqlite::Connection;
use std::{marker::PhantomData, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct EventLog(Arc<Mutex<Connection>>);

impl EventLog {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        EventLog(db)
    }
    pub async fn get_accessor(&self) -> EventLogAccessor<'_, accessor_type::All> {
        EventLogAccessor(self.0.lock().await, PhantomData::default())
    }
}
