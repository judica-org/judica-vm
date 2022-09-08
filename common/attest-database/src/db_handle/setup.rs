use super::{
    handle_type,
    sql::{CACHED, SQL_CREATE_TABLES},
    MsgDBHandle,
};

impl<'a, T> MsgDBHandle<'a, T>
where
    T: handle_type::Setup,
{
    /// Creates all the required tables for the application.
    /// Safe to call multiple times
    pub fn setup_tables(&mut self) {
        self.0.execute_batch(SQL_CREATE_TABLES).unwrap();
        // avoid accidental evictions with uncached statements
        self.0
            .set_prepared_statement_cache_capacity(CACHED.len() * 2);
        for sql  in CACHED {
            self.0.prepare_cached(sql).expect("Invalid SQL Query Detected");
        }
    }
}
