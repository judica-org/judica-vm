use super::{MsgDBHandle, handle_type};

impl<'a, T> MsgDBHandle<'a, T> where T: handle_type::Setup {
    /// Creates all the required tables for the application.
    /// Safe to call multiple times
    pub fn setup_tables(&mut self) {
        self.0
            .execute_batch(include_str!("sql/create_tables.sql"))
            .unwrap();
    }
}
