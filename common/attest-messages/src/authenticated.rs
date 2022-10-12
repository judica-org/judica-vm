// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use schemars::JsonSchema;
use serde::Serialize;

use crate::Envelope;
#[derive(Debug, Clone, Eq, PartialEq, JsonSchema, Serialize)]
#[serde(transparent)]
pub struct Authenticated<T>(pub(super) T);

impl<T> std::ops::Deref for Authenticated<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> AsRef<T> for Authenticated<T> {
    fn as_ref(&self) -> &T {
        self
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

impl<T: PartialEq> PartialEq<T> for Authenticated<T> {
    fn eq(&self, other: &T) -> bool {
        self.0 == *other
    }
}

impl PartialEq<Authenticated<Envelope>> for Envelope {
    fn eq(&self, other: &Authenticated<Envelope>) -> bool {
        *self == other.0
    }
}
