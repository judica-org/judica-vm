use rusqlite::Connection;
use std::marker::PhantomData;
use tokio::sync::MutexGuard;

pub mod accessors;
pub mod setup;

pub struct EventLogAccessor<'a, T = accessor_type::All>(
    pub MutexGuard<'a, Connection>,
    pub PhantomData<T>,
);

pub struct Setup<D: ?Sized>(PhantomData<D>);
impl<D> Default for Setup<D> {
    fn default() -> Self {
        Self(Default::default())
    }
}
pub trait SetupTrait {
    fn setup_tables(&self) -> &'static str;
    fn methods(&self) -> &'static [&'static [&'static str]];
}
pub trait DataType {}
pub mod accessor_type {
    use super::DataType;

    pub trait Insert<D: DataType> {}
    pub trait Get<D: DataType> {}
    pub trait Update<D: DataType> {}
    pub trait SetupAccessor {}

    pub struct All;
    impl<D: DataType> Insert<D> for All {}
    impl<D: DataType> Get<D> for All {}
    impl<D: DataType> Update<D> for All {}
    impl SetupAccessor for All {}
}
#[macro_export]
macro_rules! row_type_id (
    {$RowType:ident} => {
#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, Copy, serde::Serialize, serde::Deserialize, Debug)]
pub struct $RowType(i64);
impl $RowType {
    pub fn before_first_row() -> Self {
        Self(0)
    }
    pub fn inner(&self) -> i64 {
        self.0
    }
    #[cfg(test)]
    pub fn from_inner_for_test(a:i64) -> Self {
        Self(a)
    }
}
const _:() = {

    use rusqlite::types::FromSql;
    use rusqlite::types::ToSql;
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
};
    }
);

#[macro_export]
macro_rules! row_type_str (
    {$RowType:ident} => {
#[derive(PartialEq, PartialOrd, Ord, Eq, Clone, ::serde::Serialize, ::serde::Deserialize, Debug)]
pub struct $RowType(String);
const _:() = {
impl $RowType {
    pub fn inner(&self) -> &str {
        &self.0
    }
    pub fn from_inner(a:&str) -> Self {
        Self(a.to_owned())

    }
}

    use rusqlite::types::FromSql;
    use rusqlite::types::ToSql;
    impl ToSql for $RowType {
        fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
            self.0.to_sql()
        }
    }
    impl FromSql for $RowType {
        fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
            Ok($RowType(value.as_str()?.to_owned()))
        }
    }
};
    }
);
