use ruma_serde::CanonicalJsonValue;

use crate::{
    db_handle::{DataType, Setup, SetupTrait},
    row_type_id, row_type_str,
};

use self::sql::{SQL_OCCURRENCE_CACHED_QUERIES, SQL_OCCURRENCE_TABLES};

pub mod sql;

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct Occurrence {
    data: CanonicalJsonValue,
    time: i64,
    typeid: ApplicationTypeID,
}

pub trait ToOccurrence {
    fn to_data(&self) -> CanonicalJsonValue;
    fn stable_typeid(&self) -> ApplicationTypeID;
}
impl From<&dyn ToOccurrence> for Occurrence {
    fn from(t: &dyn ToOccurrence) -> Self {
        Occurrence {
            data: t.to_data(),
            time: attest_util::now(),
            typeid: t.stable_typeid(),
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
