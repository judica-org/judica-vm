use crate::db_handle::get::nonces::{extract_sk_from_envelopes, extract_sk};
use crate::db_handle::MsgDBHandle;

use super::connection::MsgDB;
use super::*;

use attest_messages::{Authenticated, CanonicalEnvelopeHash, Envelope};
use fallible_iterator::FallibleIterator;
use ruma_serde::CanonicalJsonValue;
use rusqlite::{params, Connection};

use sapio_bitcoin::secp256k1::{All, Secp256k1};
use sapio_bitcoin::KeyPair;

use std::collections::BTreeSet;
use std::sync::Arc;
use test_log::test;
use tokio::sync::Mutex;
use tracing::debug;

#[test(tokio::test)]
async fn test_setup_db() {
    let conn = setup_db().await;
    // Tests that setup can be called more than once...
    conn.get_handle().await.setup_tables();
}

#[test(tokio::test)]
async fn test_add_user() {
    let conn = setup_db().await;
    let secp = Secp256k1::new();
    let test_user = "TestUser".into();
    make_test_user(&secp, &mut conn.get_handle().await, test_user);
}

#[test(tokio::test)]
async fn test_reused_nonce() {
    let conn = setup_db().await;
    let secp = Secp256k1::new();
    let test_user = "TestUser".into();
    let mut handle = conn.get_handle().await;
    let kp = make_test_user(&secp, &mut handle, test_user);
    let envelope_1 = handle
        .wrap_message_in_envelope_for_user_by_key(CanonicalJsonValue::Null, &kp, &secp, None, None)
        .unwrap()
        .unwrap();
    let envelope_1 = envelope_1.clone().self_authenticate(&secp).unwrap();
    let envelope_2 = handle
        .wrap_message_in_envelope_for_user_by_key(
            CanonicalJsonValue::String("distinct".into()),
            &kp,
            &secp,
            None,
            None,
        )
        .unwrap()
        .unwrap();
    let envelope_2 = envelope_2.clone().self_authenticate(&secp).unwrap();
    handle
        .try_insert_authenticated_envelope(envelope_1.clone())
        .unwrap();
    handle
        .try_insert_authenticated_envelope(envelope_2.clone())
        .unwrap();
    for i in 0..2 {
        // Check that only this group is returned
        let nonces = handle.get_reused_nonces().unwrap();
        assert_eq!(nonces.len(), 1);
        let v = nonces.get(&envelope_2.inner_ref().header().key()).unwrap();
        assert_eq!(
            &v[..],
            &[envelope_1.inner_ref().clone(), envelope_2.clone().inner()][..]
        );
        let k = extract_sk_from_envelopes(envelope_1.clone(), envelope_2.clone())
            .expect("Extract successful");
        println!("{:?} {:?}", kp.secret_bytes(), k.secret_bytes());
        assert_eq!(
            k.keypair(&secp).x_only_public_key().0,
            envelope_1.header().key()
        );
        // Inserting more messages shouldn't change anything
        let envelope_i = handle
            .wrap_message_in_envelope_for_user_by_key(
                CanonicalJsonValue::String(format!("distinct-{}", i)),
                &kp,
                &secp,
                None,
                None,
            )
            .unwrap()
            .unwrap();
        let envelope_i = envelope_i.clone().self_authenticate(&secp).unwrap();
        handle
            .try_insert_authenticated_envelope(envelope_i.clone())
            .unwrap();
    }
}

fn print_db(handle: &MsgDBHandle) {
    let mut stm = handle
        .0
        .prepare("select message_id, prev_msg, hash, height, nonce from messages")
        .unwrap();
    let mut rows = stm.query([]).unwrap();
    println!("---------------------------");
    while let Ok(Some(row)) = rows.next() {
        println!(
            "   - height({}) prev({}) hash({:?}) height({}) nonce({})",
            row.get::<_, i64>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
            row.get::<_, String>(2).unwrap(),
            row.get::<_, i64>(3).unwrap(),
            row.get::<_, String>(4).unwrap(),
        );
    }
    println!("###########################");

    let mut stm = handle
        .0
        .prepare("select genesis, hash, prev_msg, connected, prev_msg_id from messages")
        .unwrap();
    let mut rows = stm.query([]).unwrap();
    while let Ok(Some(row)) = rows.next() {
        println!(
            "   - gen({:?}) me({}) prev({}) con({}) id({:?})",
            row.get::<_, String>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
            row.get::<_, String>(2).unwrap(),
            row.get::<_, bool>(3).unwrap(),
            row.get::<_, Option<i64>>(4).unwrap(),
        );
    }
}
#[test(tokio::test)]
async fn test_envelope_creation() {
    let mut all_past_tips = BTreeSet::<CanonicalEnvelopeHash>::new();
    let mut disconnected_tip = vec![];
    let mut disconnected = vec![];
    let mut kps = vec![];
    let mut final_msg = vec![];
    let verify_tip = |handle: &MsgDBHandle,
                      envelope: &Authenticated<Envelope>,
                      user_id: usize,
                      kp: KeyPair,
                      all_past_tips: &BTreeSet<CanonicalEnvelopeHash>| {
        {
            let tips = handle.get_tips_for_all_users().unwrap();
            assert_eq!(tips.len(), user_id + 1);
            assert!(tips.contains(envelope));
            assert_eq!(
                tips.iter()
                    .filter(|f| !all_past_tips.contains(&f.canonicalized_hash_ref()))
                    .count(),
                1
            );
        }
        {
            let my_tip = handle
                .get_tip_for_user_by_key(kp.x_only_public_key().0)
                .unwrap();
            assert_eq!(&my_tip, envelope.inner_ref());
        }
        {
            let known_tips = handle.get_tip_for_known_keys().unwrap();
            assert_eq!(known_tips.len(), user_id + 1);
            assert!(known_tips.contains(envelope));
            assert_eq!(
                known_tips
                    .iter()
                    .filter(|f| !all_past_tips.contains(&f.canonicalized_hash_ref()))
                    .count(),
                1
            );
        }
    };
    let secp = Secp256k1::new();
    let conn = setup_db().await;
    let mut handle = conn.get_handle().await;
    const N_USERS: usize = 10;
    for user_id in 0..N_USERS {
        let test_user = format!("Test_User_{}", user_id);
        let kp = make_test_user(&secp, &mut handle, test_user);

        let envelope_1 = handle
            .wrap_message_in_envelope_for_user_by_key(
                CanonicalJsonValue::Null,
                &kp,
                &secp,
                None,
                None,
            )
            .unwrap()
            .unwrap();
        let envelope_1 = envelope_1.clone().self_authenticate(&secp).unwrap();
        handle
            .try_insert_authenticated_envelope(envelope_1.clone())
            .unwrap();
        verify_tip(&handle, &envelope_1, user_id, kp, &all_past_tips);

        let envelope_2 = handle
            .wrap_message_in_envelope_for_user_by_key(
                CanonicalJsonValue::Null,
                &kp,
                &secp,
                None,
                None,
            )
            .unwrap()
            .unwrap();
        let envelope_2 = envelope_2.clone().self_authenticate(&secp).unwrap();
        handle
            .try_insert_authenticated_envelope(envelope_2.clone())
            .unwrap();
        verify_tip(&handle, &envelope_2, user_id, kp, &all_past_tips);

        let mut envs: Vec<(CanonicalEnvelopeHash, Authenticated<Envelope>)> = vec![];
        let special_idx = 5;
        for i in 0..10isize {
            let envelope_disconnected = handle
                .wrap_message_in_envelope_for_user_by_key(
                    CanonicalJsonValue::Null,
                    &kp,
                    &secp,
                    None,
                    envs.get((i - 1) as usize).map(|a| a.1.inner_ref().clone()),
                )
                .unwrap()
                .unwrap();
            let envelope_disconnected = envelope_disconnected
                .clone()
                .self_authenticate(&secp)
                .unwrap();
            envs.push((
                envelope_disconnected.inner_ref().canonicalized_hash_ref(),
                envelope_disconnected.clone(),
            ));
            if i != special_idx {
                println!("Inserting i={}", i);
                handle
                    .try_insert_authenticated_envelope(envelope_disconnected.clone())
                    .unwrap();
            } else {
                println!("Skipping i={}", i);
            }
            let idx = if i >= special_idx { special_idx - 1 } else { i };
            let check_envelope = &envs[idx as usize].1;

            verify_tip(&handle, &check_envelope, user_id, kp, &all_past_tips);
            if i > special_idx {
                let tips = handle.get_disconnected_tip_for_known_keys().unwrap();
                assert_eq!(tips.len(), user_id + 1);
                assert!(tips.contains(&envs[special_idx as usize + 1].1));
            }
        }
        all_past_tips.insert(envs[(special_idx - 1) as usize].0);
        disconnected.push(envs[special_idx as usize].1.clone());
        disconnected_tip.push(envs[(special_idx + 1) as usize].1.clone());
        final_msg.push(envs[9].0);
        kps.push(kp);
    }

    for user_id in 0..N_USERS {
        // handle.drop_message_by_hash(envs[5].0).unwrap();

        handle
            .try_insert_authenticated_envelope(disconnected[user_id].clone())
            .unwrap();
        {
            let tips = handle.get_disconnected_tip_for_known_keys().unwrap();
            assert_eq!(tips.len(), N_USERS);
            assert!(tips.contains(&disconnected_tip[user_id]));
        }
        {
            let my_tip = handle
                .get_tip_for_user_by_key(kps[user_id].x_only_public_key().0)
                .unwrap();
            assert_eq!(
                my_tip.canonicalized_hash_ref(),
                disconnected[user_id].inner_ref().canonicalized_hash_ref()
            );
        }
        {
            let known_tips = handle.get_tip_for_known_keys().unwrap();
            assert_eq!(known_tips.len(), N_USERS);
            assert!(known_tips.contains(&disconnected[user_id]));
        }
    }

    let tips_attached = handle.attach_tips().unwrap();
    debug!(tips_attached);
    let tips_attached = handle.attach_tips().unwrap();
    assert_eq!(tips_attached, 0);

    let known_tips: Vec<_> = handle
        .get_tip_for_known_keys()
        .unwrap()
        .iter()
        .map(|t| t.canonicalized_hash_ref())
        .collect();
    for user_id in 0..N_USERS {
        {
            let my_tip = handle
                .get_tip_for_user_by_key(kps[user_id].x_only_public_key().0)
                .unwrap();
            assert_eq!(my_tip.canonicalized_hash_ref(), final_msg[user_id]);
        }
        {
            assert_eq!(known_tips.len(), N_USERS);
            assert!(known_tips.contains(&final_msg[user_id]));
        }
    }

    let kp_2 = make_test_user(&secp, &mut handle, "TestUser2".into());

    let envelope_3 = handle
        .wrap_message_in_envelope_for_user_by_key(
            CanonicalJsonValue::Null,
            &kp_2,
            &secp,
            None,
            None,
        )
        .unwrap()
        .unwrap();
    let envelope_3 = envelope_3.clone().self_authenticate(&secp).unwrap();
    handle
        .try_insert_authenticated_envelope(envelope_3.clone())
        .unwrap();

    {
        let known_tips = handle.get_tip_for_known_keys().unwrap();
        assert_eq!(known_tips.len(), kps.len() + 1);
        let mut tip_hashes: Vec<_> = known_tips
            .iter()
            .map(|t| t.canonicalized_hash_ref())
            .collect();
        tip_hashes.sort();
        final_msg.push(envelope_3.inner_ref().canonicalized_hash_ref());
        final_msg.sort();
        assert_eq!(&tip_hashes[..], &final_msg[..]);
    }
}

fn make_test_user(
    secp: &Secp256k1<All>,
    handle: &mut db_handle::MsgDBHandle<'_>,
    name: String,
) -> KeyPair {
    let (kp, nonce, envelope) = generate_new_user(secp).unwrap();
    let _u = handle.save_keypair(kp).unwrap();
    let genesis = envelope.self_authenticate(secp).unwrap();
    handle
        .insert_user_by_genesis_envelope(name, genesis)
        .unwrap();
    handle
        .save_nonce_for_user_by_key(nonce, secp, kp.x_only_public_key().0)
        .unwrap();
    kp
}

async fn setup_db() -> MsgDB {
    let conn = MsgDB::new(Arc::new(Mutex::new(Connection::open_in_memory().unwrap())));
    conn.get_handle().await.setup_tables();
    conn
}
#[test(tokio::test)]
async fn test_tables() {
    let conn = setup_db().await;
    let handle = conn.get_handle().await;
    let mut it = handle
        .0
        .prepare(
            "SELECT name FROM sqlite_schema
        WHERE type='table'
        AND name NOT LIKE 'sqlite_%'
        ORDER BY name;
        ",
        )
        .unwrap();
    let vit: Vec<_> = it
        .query(params![])
        .unwrap()
        .map(|r| r.get::<_, String>(0))
        .collect()
        .unwrap();
    assert_eq!(
        vec![
            "chain_commit_group_members",
            "chain_commit_groups",
            "hidden_services",
            "message_nonces",
            "messages",
            "private_keys",
            "users"
        ],
        vit
    )
}
