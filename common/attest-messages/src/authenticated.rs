use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Authenticated<T>(pub(super) T);

impl<T> std::ops::Deref for Authenticated<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> Authenticated<T> {
    pub fn inner(self) -> T {
        self.0
    }

    pub fn inner_ref(&self) -> &T {
        &self.0
    }
}
