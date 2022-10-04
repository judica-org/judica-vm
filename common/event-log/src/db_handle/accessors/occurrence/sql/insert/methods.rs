use std::{
    error::Error,
    fmt::{Debug, Display},
};

use rusqlite::{named_params, ErrorCode, Transaction};

use crate::{
    db_handle::{
        accessor_type::Insert,
        accessors::{
            occurrence::{Occurrence, OccurrenceID, ToOccurrence},
            occurrence_group::OccurrenceGroupID,
        },
        EventLogAccessor,
    },
    sql_serializers::SqlJsonRef,
};

use super::SQL_NEW_OCCURRENCE;

impl<'a, T> EventLogAccessor<'a, T>
where
    T: Insert<Occurrence>,
{
    pub fn insert_new_occurrence_now_from<I>(
        &self,
        group_id: OccurrenceGroupID,
        data: &I,
    ) -> Result<Result<OccurrenceID, Idempotent>, rusqlite::Error>
    where
        I: ToOccurrence,
    {
        let occurrence = Occurrence::from(data);
        self.insert_occurrence(group_id, &occurrence)
    }

    pub fn insert_occurrence(
        &self,
        group_id: OccurrenceGroupID,
        occurrence: &Occurrence,
    ) -> Result<Result<OccurrenceID, Idempotent>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_NEW_OCCURRENCE)?;
        match stmt.insert(named_params! {
            ":data": SqlJsonRef(&occurrence.data),
            ":time": occurrence.time,
            ":typeid": occurrence.typeid,
            ":group_id": group_id
        }) {
            Ok(q) => Ok(Ok(OccurrenceID(q))),
            Err(e) => match e {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error {
                        code: ErrorCode::ConstraintViolation,
                        extended_code: crate::sql_error::SQLITE_CONSTRAINT_UNIQUE,
                    },
                    _,
                ) => Ok(Err(Idempotent::AlreadyExists)),
                e => Err(e),
            },
        }
    }

    pub fn insert_new_occurrence_now_from_txn<'c, I>(
        &'c mut self,
        group_id: OccurrenceGroupID,
        data: &I,
    ) -> Result<Result<(OccurrenceID, Transaction<'c>), Idempotent>, rusqlite::Error>
    where
        I: ToOccurrence,
    {
        let txn = self.0.transaction()?;
        let occurrence = Occurrence::from(data);
        insert_occurrence_txn(txn, group_id, &occurrence)
    }
}

pub fn insert_occurrence_txn<'conn>(
    txn: Transaction<'conn>,
    group_id: OccurrenceGroupID,
    occurrence: &Occurrence,
) -> Result<Result<(OccurrenceID, Transaction<'conn>), Idempotent>, rusqlite::Error> {
    let mut stmt = txn.prepare_cached(SQL_NEW_OCCURRENCE)?;
    match stmt.insert(named_params! {
        ":data": SqlJsonRef(&occurrence.data),
        ":time": occurrence.time,
        ":typeid": occurrence.typeid,
        ":group_id": group_id
    }) {
        Ok(q) => {
            drop(stmt);
            Ok(Ok((OccurrenceID(q), txn)))
        },
        Err(e) => {
            drop(stmt);
            match e {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error {
                        code: ErrorCode::ConstraintViolation,
                        extended_code: crate::sql_error::SQLITE_CONSTRAINT_UNIQUE,
                    },
                    _,
                ) => Ok(Err(Idempotent::AlreadyExists)),
                e => Err(e),
            }
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub enum Idempotent {
    AlreadyExists,
}
impl Display for Idempotent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}
impl Error for Idempotent {}
