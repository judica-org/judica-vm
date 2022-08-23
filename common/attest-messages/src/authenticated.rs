use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Authenticated<T>(pub(super) T);
impl<T> Authenticated<T> {
    pub fn inner(self) -> T {
        self.0
    }

    pub fn inner_ref(&self) -> &T {
        &self.0
    }
}