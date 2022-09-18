use crate::db_handle::{
    handle_type::{Get, Insert},
    EventLogAccessor,
};

use super::Occurrence;

impl<'a, T> EventLogAccessor<'a, T> where T: Get<Occurrence> + Insert<Occurrence> {


}
