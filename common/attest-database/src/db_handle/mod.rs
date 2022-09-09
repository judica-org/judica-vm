use std::marker::PhantomData;

use super::sql_serializers::{self};
use rusqlite::{types::FromSql, Connection, ToSql};
use serde::{Deserialize, Serialize};
use tokio::sync::MutexGuard;

pub mod create;
pub mod get;
pub mod insert;
pub mod setup;
pub mod update;
pub mod sql;

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
macro_rules! row_type (
    {$RowType:ident} => {
#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, Serialize, Deserialize, Debug)]
pub struct $RowType(i64);
impl ToSql for $RowType {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}
impl FromSql for $RowType {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        value.as_i64().map($RowType)
    }
}
    }
);

row_type!(ChainCommitGroupID);
row_type!(MessageID);
