use tracing::trace;

use super::{
    handle_type,
    sql::{CACHED, SQL_CREATE_TABLES},
    MsgDBHandle,
};

impl<T> MsgDBHandle<T>
where
    T: handle_type::Setup,
{
    /// Creates all the required tables for the application.
    /// Safe to call multiple times
    pub fn setup_tables(&mut self) {
        self.0
            .execute_batch(SQL_CREATE_TABLES)
            .expect("Table Setup Failed");
        // avoid accidental evictions with uncached statements
        self.0
            .set_prepared_statement_cache_capacity(CACHED.len() * 2);
        for (i, sql) in CACHED.iter().enumerate() {
            trace!(?sql, i, "Preparing Cached SQL");
            self.0
                .prepare_cached(sql)
                .expect("Invalid SQL Query Detected");
        }
    }
}
