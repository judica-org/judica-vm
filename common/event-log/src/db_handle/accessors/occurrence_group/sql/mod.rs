pub mod create;
pub mod get;
pub mod insert;
pub mod tables;
pub mod update;

pub const SQL_OCCURRENCE_CACHED_QUERIES: &[&[&str]] = &[
    get::MANIFEST,
    insert::MANIFEST,
    create::MANIFEST,
    tables::MANIFEST,
    update::MANIFEST,
];
