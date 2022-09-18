use crate::db_handle::{
    accessor_type::{Get, Insert},
    accessors::occurrence_group::OccurrenceGroup,
    EventLogAccessor,
};

impl<'a, T> EventLogAccessor<'a, T> where T: Get<OccurrenceGroup> + Insert<OccurrenceGroup> {}
