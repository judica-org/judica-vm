mod methods;
pub const SQL_GET_OCCURRENCES_FOR_GROUP: &str = include_str!("occurrence_for_group.sql");
pub const SQL_GET_OCCURRENCE_AFTER_ID: &str = include_str!("occurrence_for_group_after_id.sql");
pub const SQL_GET_OCCURRENCE_BY_ID: &str = include_str!("occurrence_by_id.sql");
pub const MANIFEST: &[&str] = &[
    SQL_GET_OCCURRENCE_BY_ID,
    SQL_GET_OCCURRENCES_FOR_GROUP,
    SQL_GET_OCCURRENCE_AFTER_ID,
];
