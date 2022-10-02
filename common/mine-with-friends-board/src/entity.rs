use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::num::ParseIntError;

/// an EntityID is just a "pointer" we assign to all different types of things in our game, e.g.
/// - Users
/// - Token Contracts
/// - NFTs
/// - etc
///
/// EntityIDs are global and unique within the game state
#[derive(
    Eq, Ord, PartialEq, PartialOrd, Clone, Copy, Serialize, Deserialize, JsonSchema, Debug,
)]
#[serde(into = "String")]
#[serde(try_from = "String")]
pub struct EntityID(#[schemars(with = "String")] pub u64);

impl EntityID {
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl TryFrom<&str> for EntityID {
    type Error = ParseIntError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        u64::from_str_radix(value, 16).map(EntityID)
    }
}
impl TryFrom<String> for EntityID {
    type Error = ParseIntError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        u64::from_str_radix(&value, 16).map(EntityID)
    }
}

impl From<EntityID> for String {
    fn from(a: EntityID) -> Self {
        format!("{:x}", a.0)
    }
}

/// Allocator which can assign IDs sequentially
#[derive(Serialize)]
pub struct EntityIDAllocator(pub u64);

impl EntityIDAllocator {
    /// Creates a new instance with a "easy to see" start number, which helps in
    /// debugging
    pub fn new() -> Self {
        Self(0x000c0de0000)
    }
    pub(crate) fn make(&mut self) -> EntityID {
        self.0 += 1;
        EntityID(self.0)
    }
}
