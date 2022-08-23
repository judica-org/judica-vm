use super::db_handle::MsgDBHandle;
use crate::db_handle::handle_type;
use rusqlite::Connection;
use std::{marker::PhantomData, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct MsgDB(Arc<Mutex<Connection>>);

impl MsgDB {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        MsgDB(db)
    }
    pub async fn get_handle<'a>(&'a self) -> MsgDBHandle<'a, handle_type::All> {
        MsgDBHandle(self.0.lock().await, PhantomData::default())
    }
}
