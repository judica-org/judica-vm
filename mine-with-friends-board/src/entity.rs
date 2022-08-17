use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::num::ParseIntError;

#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct EntityID(pub u64);

impl EntityID {
    pub fn is_valid(&self) -> bool {
        self.0 != 0
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

#[derive(Serialize)]
pub struct EntityIDAllocator(pub u64);

impl EntityIDAllocator {
    pub(crate) fn make(&mut self) -> EntityID {
        self.0 += 1;
        EntityID(self.0)
    }
}
