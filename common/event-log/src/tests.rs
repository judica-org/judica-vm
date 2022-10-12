// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    connection::EventLog,
    db_handle::accessors::occurrence::{ApplicationTypeID, Occurrence, OccurrenceID, ToOccurrence},
};
use rusqlite::Connection;
use std::sync::Arc;
use test_log::test;
use tokio::sync::Mutex;

async fn setup_test_db() -> EventLog {
    let conn = EventLog::new(Arc::new(Mutex::new(Connection::open_in_memory().unwrap())));
    conn.get_accessor().await.setup_tables();
    conn
}

#[test(tokio::test)]
async fn test_setup_db() {
    let conn = setup_test_db().await;
    // Tests that setup can be called more than once...
    conn.get_accessor().await.setup_tables();
}

struct StringOccurrence(String);
impl ToOccurrence for StringOccurrence {
    fn to_data(&self) -> ruma_serde::CanonicalJsonValue {
        ruma_serde::CanonicalJsonValue::String(self.0.clone())
    }
    fn unique_tag(&self) -> Option<String> {
        None
    }

    fn stable_typeid() -> crate::db_handle::accessors::occurrence::ApplicationTypeID {
        ApplicationTypeID::from_inner("StringOccurrence")
    }
}
#[test(tokio::test)]
async fn test_db_basic_function() {
    let conn = setup_test_db().await;
    // Tests that setup can be called more than once...
    conn.get_accessor().await.setup_tables();

    let accessor = conn.get_accessor().await;
    let groups = accessor.get_all_occurrence_groups().unwrap();
    assert_eq!(groups.len(), 0);

    let group_one = accessor
        .insert_new_occurrence_group(&"Test".into())
        .unwrap();

    assert_eq!(
        "Test",
        accessor.get_occurrence_group_by_id(group_one).unwrap()
    );

    assert_eq!(
        group_one,
        accessor
            .get_occurrence_group_by_key(&"Test".into())
            .unwrap()
    );

    assert_eq!(
        vec![(group_one, "Test".into())],
        accessor.get_all_occurrence_groups().unwrap()
    );

    assert_eq!(
        accessor.get_occurrences_for_group(group_one).unwrap(),
        vec![]
    );

    assert_eq!(
        accessor
            .get_occurrences_for_group_after_id(
                group_one,
                OccurrenceID::from_inner_for_test(i64::MIN)
            )
            .unwrap(),
        vec![]
    );

    assert!(accessor
        .get_occurrence(OccurrenceID::from_inner_for_test(1234))
        .is_err(),);

    let occur_once = StringOccurrence("First Message".into());
    let occurrence_one = accessor
        .insert_new_occurrence_now_from(group_one, &occur_once)
        .unwrap()
        .unwrap();
    let o = StringOccurrence("Twice".into());
    let repeat = Occurrence::from(&o);
    let occurrence_two = accessor
        .insert_occurrence(group_one, &repeat)
        .unwrap()
        .unwrap();
    let occurrence_three = accessor
        .insert_occurrence(group_one, &repeat)
        .unwrap()
        .unwrap();

    let occurrences = vec![
        (occurrence_one, (&occur_once).into()),
        (occurrence_two, repeat.clone()),
        (occurrence_three, repeat.clone()),
    ];
    assert_eq!(
        accessor.get_occurrences_for_group(group_one).unwrap(),
        occurrences,
    );

    assert_eq!(accessor.get_occurrence(occurrence_two).unwrap(), repeat);
    assert_eq!(
        accessor
            .get_occurrences_for_group_after_id(group_one, occurrence_one)
            .unwrap(),
        occurrences[1..]
    );

    assert_eq!(
        accessor
            .get_occurrences_for_group_after_id(group_one, occurrence_two)
            .unwrap(),
        occurrences[2..]
    );

    assert_eq!(
        accessor
            .get_occurrences_for_group_after_id(group_one, occurrence_three)
            .unwrap(),
        occurrences[3..]
    );
}
