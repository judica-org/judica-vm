use super::Occurrence;
use crate::db_handle::{
    handle_type::{Get, Insert},
    EventLogAccessor,
};

impl<'a, T> EventLogAccessor<'a, T> where T: Get<Occurrence> + Insert<Occurrence> {}
