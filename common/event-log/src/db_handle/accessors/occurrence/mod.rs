use std::fmt::Display;

use ruma_serde::CanonicalJsonValue;
use serde::Deserialize;

use crate::{
    db_handle::{DataType, Setup, SetupTrait},
    row_type_id, row_type_str,
};

use self::sql::{SQL_OCCURRENCE_CACHED_QUERIES, SQL_OCCURRENCE_TABLES};

pub mod sql;

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Occurrence {
    pub data: CanonicalJsonValue,
    pub time: i64,
    pub typeid: ApplicationTypeID,
    pub unique_tag: Option<String>,
}

#[derive(Debug)]
pub enum OccurrenceConversionError {
    DeserializationError(serde_json::Error),
    TypeidMismatch {
        expected: ApplicationTypeID,
        got: ApplicationTypeID,
    },
}
impl Display for OccurrenceConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for OccurrenceConversionError {}
pub trait ToOccurrence {
    fn to_data(&self) -> CanonicalJsonValue;
    fn stable_typeid() -> ApplicationTypeID
    where
        Self: Sized;
    fn unique_tag(&self) -> Option<String>;
    fn from_occurrence(occurrence: Occurrence) -> Result<Self, OccurrenceConversionError>
    where
        Self: Sized + for<'de> Deserialize<'de>,
    {
        let v: Self = serde_json::from_value(occurrence.data.into())
            .map_err(OccurrenceConversionError::DeserializationError)?;
        if occurrence.typeid != Self::stable_typeid() {
            return Err(OccurrenceConversionError::TypeidMismatch {
                expected: occurrence.typeid,
                got: Self::stable_typeid(),
            });
        }
        Ok(v)
    }
}
impl<T: ToOccurrence> From<&T> for Occurrence {
    fn from(t: &T) -> Self {
        Occurrence {
            data: t.to_data(),
            time: attest_util::now(),
            typeid: T::stable_typeid(),
            unique_tag: t.unique_tag(),
        }
    }
}

impl DataType for Occurrence {}
impl SetupTrait for Setup<Occurrence> {
    fn setup_tables(&self) -> &'static str {
        SQL_OCCURRENCE_TABLES
    }
    fn methods(&self) -> &'static [&'static [&'static str]] {
        SQL_OCCURRENCE_CACHED_QUERIES
    }
}

row_type_id!(OccurrenceID);
row_type_str!(ApplicationTypeID);
