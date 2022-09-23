use crate::tokens::TokenPointer;

/// Convenience type to wrap a u128
pub(crate) type Price = u128;
/// More convenient name in some contexts
pub type Currency = TokenPointer;

pub type Watts = u128;
pub type Location = (i64, i64);
pub type ForSale = bool;
pub type HasMiners = bool;
