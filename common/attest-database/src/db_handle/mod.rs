use std::marker::PhantomData;

use super::sql_serializers::{self};
use rusqlite::Connection;
use tokio::sync::MutexGuard;

pub mod create;
pub mod get;
pub mod insert;
pub mod setup;
pub mod update;
pub struct MsgDBHandle<'a, T = handle_type::All>(
    pub MutexGuard<'a, Connection>,
    pub PhantomData<T>,
);

pub enum ConsistentMessages {
    AllMessagesNotReady,
}

pub mod handle_type {

    pub trait Insert {}
    pub trait Get {}
    pub trait Setup {}
    pub trait Update {}
    pub struct All;
    impl Insert for All {}
    impl Get for All {}
    impl Setup for All {}
    impl Update for All {}
}
