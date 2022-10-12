// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use tracing::trace;

use super::{
    accessor_type::SetupAccessor,
    accessors::{occurrence::Occurrence, occurrence_group::OccurrenceGroup},
    EventLogAccessor, Setup, SetupTrait,
};

impl<'a, T> EventLogAccessor<'a, T>
where
    T: SetupAccessor,
{
    /// Creates all the required tables for the application.
    /// Safe to call multiple times
    pub fn setup_tables(&mut self) {
        let all_types: &[&dyn SetupTrait] = &[
            &Setup::<OccurrenceGroup>::default(),
            &Setup::<Occurrence>::default(),
        ];
        let all_tables: String = String::from("PRAGMA foreign_keys = ON;\n")
            + &all_types
                .iter()
                .map(|m| m.setup_tables())
                .collect::<Vec<_>>()
                .join("\n")
            + "\nPRAGMA journal_mode = WAL;";
        self.0
            .execute_batch(&all_tables)
            .expect("Table Setup Failed");
        // avoid accidental evictions with uncached statements
        let n_methods: usize = all_types.iter().map(|m| m.methods().len()).sum();
        let all_methods = all_types
            .iter()
            .flat_map(|m| m.methods())
            .cloned()
            .flatten()
            .cloned();
        self.0.set_prepared_statement_cache_capacity(n_methods * 2);
        for (i, sql) in all_methods.enumerate() {
            trace!(?sql, i, "Preparing Cached SQL");
            self.0
                .prepare_cached(sql)
                .expect("Invalid SQL Query Detected");
        }
    }
}
