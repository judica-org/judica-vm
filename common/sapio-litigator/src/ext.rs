// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sapio::contract::{abi::continuation::ContinuationPoint, Compiled};

pub(crate) trait CompiledExt {
    fn continuation_points<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContinuationPoint> + 'a>;
}

// TODO: Do away with allocations?
impl CompiledExt for Compiled {
    fn continuation_points<'a>(&'a self) -> Box<dyn Iterator<Item = &'a ContinuationPoint> + 'a> {
        Box::new(
            self.continue_apis.values().chain(
                self.suggested_txs
                    .values()
                    .chain(self.ctv_to_tx.values())
                    .flat_map(|x| &x.outputs)
                    .flat_map(|x| x.contract.continuation_points()),
            ),
        )
    }
}
