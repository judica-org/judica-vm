use serde::{Serialize, Deserialize};
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Authenticated<T>(pub T);
impl<T> Authenticated<T> {
    pub fn inner(self) -> T {
        self.0
    }

    pub fn inner_ref(&self) -> &T {
        &self.0
    }
}
