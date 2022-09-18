pub use get::*;
pub use insert::*;
pub use tables::*;
pub use update::*;
pub mod insert;

pub mod update {}

pub mod get;
pub mod tables;

pub const SQL_OCCURRENCE_CACHED_QUERIES: &[&[&str]] = &[get::MANIFEST, insert::MANIFEST];
