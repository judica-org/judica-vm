use serde::{Deserialize, Serialize};
use std::num::ParseIntError;

#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Copy, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct EntityID(pub u128);

impl TryFrom<String> for EntityID {
    type Error = ParseIntError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        u128::from_str_radix(&value, 16).map(EntityID)
    }
}

impl From<EntityID> for String {
    fn from(a: EntityID) -> Self {
        format!("{:x}", a.0)
    }
}

#[derive(Serialize)]
pub struct EntityIDAllocator(pub u128);

impl EntityIDAllocator {
    pub(crate) fn make(&mut self) -> EntityID {
        self.0 += 1;
        EntityID(self.0)
    }
}
