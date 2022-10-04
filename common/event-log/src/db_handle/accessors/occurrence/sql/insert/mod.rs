mod methods;
pub use methods::Idempotent;
pub const SQL_NEW_OCCURRENCE: &str = include_str!("new_occurrence.sql");
pub const MANIFEST: &[&str] = &[SQL_NEW_OCCURRENCE];
