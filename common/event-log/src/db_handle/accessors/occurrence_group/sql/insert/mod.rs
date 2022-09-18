mod methods;
pub const SQL_NEW_OCCURRENCE_GROUP: &str = include_str!("new_occurrence_group.sql");
pub const MANIFEST: &[&str] = &[SQL_NEW_OCCURRENCE_GROUP];
