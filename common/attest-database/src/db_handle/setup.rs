use super::{handle_type, MsgDBHandle};

const SQL_CREATE_TABLES: &'static str = concat!(
    "PRAGMA foreign_keys = ON;",
    include_str!("sql/tables/users.sql"),
    include_str!("sql/tables/messages.sql"),
    include_str!("sql/tables/nonces.sql"),
    include_str!("sql/tables/private_keys.sql"),
    include_str!("sql/tables/chain_commit_groups.sql"),
    include_str!("sql/tables/chain_commit_group_members.sql"),
    include_str!("sql/tables/hidden_services.sql"),
    include_str!("sql/triggers/messages/connect_gap_parent.sql"),
    "PRAGMA journal_mode = WAL;"
);

impl<'a, T> MsgDBHandle<'a, T>
where
    T: handle_type::Setup,
{
    /// Creates all the required tables for the application.
    /// Safe to call multiple times
    pub fn setup_tables(&mut self) {
        self.0.execute_batch(SQL_CREATE_TABLES).unwrap();
    }
}
