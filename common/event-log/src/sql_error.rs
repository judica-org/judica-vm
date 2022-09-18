/// Constant for Unique Contraint Violation
/// Yes, pattern matching works.
///```
/// use std::os::raw::c_int;
/// const X: c_int = 0;
/// struct Y {
///     val: c_int,
/// }
/// match (Y { val: 1 }) {
///     Y { val: X } => panic!("bad"),
///     Y { val: b } => println!("good"),
/// }
/// match (Y { val: 0 }) {
///     Y { val: X } => println!("good"),
///     Y { val: b } => panic!("bad"),
/// }
///```
pub use rusqlite::ffi::{
    SQLITE_CONSTRAINT_CHECK, SQLITE_CONSTRAINT_NOTNULL, SQLITE_CONSTRAINT_UNIQUE,
};

#[must_use]
#[derive(Debug)]
#[repr(i32)]
pub enum SqliteFail {
    SqliteConstraintUnique = SQLITE_CONSTRAINT_UNIQUE,
    SqliteConstraintNotNull = SQLITE_CONSTRAINT_NOTNULL,
    SqliteConstraintCheck = SQLITE_CONSTRAINT_CHECK,
}

impl std::fmt::Display for SqliteFail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for SqliteFail {}
