use super::MsgDBHandle;


impl<'a> MsgDBHandle<'a> {
    /// Creates all the required tables for the application.
    /// Safe to call multiple times
    pub fn setup_tables(&mut self) {
        self.0
            .execute_batch(include_str!("sql/create_tables.sql"))
            .unwrap();
    }
}