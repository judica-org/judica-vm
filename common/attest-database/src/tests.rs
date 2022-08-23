use crate::db_handle::MsgDBHandle;

use super::connection::MsgDB;
use super::*;
use attest_messages::nonce::PrecomittedNonce;
use attest_messages::{Authenticated, CanonicalEnvelopeHash, Envelope, Header, Unsigned};
use fallible_iterator::FallibleIterator;
use rusqlite::{params, Connection};

use sapio_bitcoin::secp256k1::{rand, All, Secp256k1};
use sapio_bitcoin::KeyPair;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_setup_db() {
    let conn = setup_db().await;
    // Tests that setup can be called more than once...
    conn.get_handle().await.setup_tables();
}

#[tokio::test]
async fn test_add_user() {
    let conn = setup_db().await;
    let secp = Secp256k1::new();
    let test_user = "TestUser".into();
    make_test_user(&secp, &conn.get_handle().await, test_user);
}

#[tokio::test]
async fn test_reused_nonce() {
    let conn = setup_db().await;
    let secp = Secp256k1::new();
    let test_user = "TestUser".into();
    let handle = conn.get_handle().await;
    let kp = make_test_user(&secp, &handle, test_user);
    let envelope_1 = handle
        .wrap_message_in_envelope_for_user_by_key(Value::Null, &kp, &secp, None)
        .unwrap()
        .unwrap();
    let envelope_1 = envelope_1.clone().self_authenticate(&secp).unwrap();
    let envelope_2 = handle
        .wrap_message_in_envelope_for_user_by_key(json!("distinct"), &kp, &secp, None)
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
        print_db(&handle);
        let v = nonces.get(&envelope_2.inner_ref().header.key).unwrap();
        assert_eq!(
            &v[..],
            &[envelope_1.inner_ref().clone(), envelope_2.clone().inner()][..]
        );
        // Inserting more messages shouldn't change anything
        let envelope_i = handle
            .wrap_message_in_envelope_for_user_by_key(json!({ "distinct": i }), &kp, &secp, None)
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
        .prepare("select genesis, hash from messages")
        .unwrap();
    let mut rows = stm.query([]).unwrap();
    while let Ok(Some(row)) = rows.next() {
        println!(
            "   - gen({:?}) me({})",
            row.get::<_, String>(0).unwrap(),
            row.get::<_, String>(1).unwrap(),
        );
    }
}
#[tokio::test]
async fn test_envelope_creation() {
    let conn = setup_db().await;
    let secp = Secp256k1::new();
    let test_user = "TestUser".into();
    let handle = conn.get_handle().await;
    let kp = make_test_user(&secp, &handle, test_user);

    print_db(&handle);
    let envelope_1 = handle
        .wrap_message_in_envelope_for_user_by_key(Value::Null, &kp, &secp, None)
        .unwrap()
        .unwrap();
    let envelope_1 = envelope_1.clone().self_authenticate(&secp).unwrap();
    handle
        .try_insert_authenticated_envelope(envelope_1.clone())
        .unwrap();

    {
        let tips = handle.get_tips_for_all_users().unwrap();
        assert_eq!(tips.len(), 1);
        assert_eq!(&tips[0], envelope_1.inner_ref());
    }
    {
        let my_tip = handle
            .get_tip_for_user_by_key(kp.x_only_public_key().0)
            .unwrap();
        assert_eq!(&my_tip, envelope_1.inner_ref());
    }
    print_db(&handle);
    {
        let known_tips = handle.get_tip_for_known_keys().unwrap();
        assert_eq!(known_tips.len(), 1);
        assert_eq!(&known_tips[0], envelope_1.inner_ref());
    }

    let envelope_2 = handle
        .wrap_message_in_envelope_for_user_by_key(Value::Null, &kp, &secp, None)
        .unwrap()
        .unwrap();
    let envelope_2 = envelope_2.clone().self_authenticate(&secp).unwrap();
    handle
        .try_insert_authenticated_envelope(envelope_2.clone())
        .unwrap();
    {
        let tips = handle.get_tips_for_all_users().unwrap();
        assert_eq!(tips.len(), 1);
        assert_eq!(&tips[0], envelope_2.inner_ref());
    }
    {
        let my_tip = handle
            .get_tip_for_user_by_key(kp.x_only_public_key().0)
            .unwrap();
        assert_eq!(&my_tip, envelope_2.inner_ref());
    }
    {
        let known_tips = handle.get_tip_for_known_keys().unwrap();
        assert_eq!(known_tips.len(), 1);
        assert_eq!(&known_tips[0], envelope_2.inner_ref());
    }
    print_db(&handle);

    let mut envs: Vec<(CanonicalEnvelopeHash, Authenticated<Envelope>)> = vec![];
    let special_idx = 5;
    for i in 0..10isize {
        let envelope_disconnected = handle
            .wrap_message_in_envelope_for_user_by_key(
                Value::Null,
                &kp,
                &secp,
                envs.get((i - 1) as usize).map(|a| a.1.inner_ref().clone()),
            )
            .unwrap()
            .unwrap();
        let envelope_disconnected = envelope_disconnected
            .clone()
            .self_authenticate(&secp)
            .unwrap();
        envs.push((
            envelope_disconnected
                .inner_ref()
                .canonicalized_hash_ref()
                .unwrap(),
            envelope_disconnected.clone(),
        ));
        if i != special_idx {
            handle
                .try_insert_authenticated_envelope(envelope_disconnected.clone())
                .unwrap();
            {
                let tips = handle.get_tips_for_all_users().unwrap();
                assert_eq!(tips.len(), 1);
                assert_eq!(&tips[0], envelope_disconnected.inner_ref());
            }
            {
                let my_tip = handle
                    .get_tip_for_user_by_key(kp.x_only_public_key().0)
                    .unwrap();
                assert_eq!(&my_tip, envelope_disconnected.inner_ref());
            }
            {
                let known_tips = handle.get_tip_for_known_keys().unwrap();
                assert_eq!(known_tips.len(), 1);
                assert_eq!(&known_tips[0], envelope_disconnected.inner_ref());
            }
        }
    }

    {
        // handle.drop_message_by_hash(envs[5].0).unwrap();
        print_db(&handle);
        {
            let tips = handle.get_disconnected_tip_for_known_keys().unwrap();
            assert_eq!(tips.len(), 1);
            assert_eq!(&tips[0], envs[6].1.inner_ref());
        }
        {
            let my_tip = handle
                .get_tip_for_user_by_key(kp.x_only_public_key().0)
                .unwrap();
            assert_eq!(my_tip.canonicalized_hash_ref().unwrap(), envs[9].0);
        }
        {
            let known_tips = handle.get_tip_for_known_keys().unwrap();
            assert_eq!(known_tips.len(), 1);
            assert_eq!(known_tips[0].canonicalized_hash_ref().unwrap(), envs[9].0);
        }
        handle
            .try_insert_authenticated_envelope(envs[5].1.clone())
            .unwrap();
    }

    print_db(&handle);
    let kp_2 = make_test_user(&secp, &handle, "TestUser2".into());

    let envelope_3 = handle
        .wrap_message_in_envelope_for_user_by_key(Value::Null, &kp_2, &secp, None)
        .unwrap()
        .unwrap();
    let envelope_3 = envelope_3.clone().self_authenticate(&secp).unwrap();
    handle
        .try_insert_authenticated_envelope(envelope_3.clone())
        .unwrap();

    {
        let mut known_tips = handle.get_tip_for_known_keys().unwrap();
        assert_eq!(known_tips.len(), 2);
        known_tips.sort_by_key(|t| t.header.key);
        let mut presumed_tips = [
            envelope_3.inner_ref().clone(),
            envs[9].1.inner_ref().clone(),
        ];
        presumed_tips.sort_by_key(|p| p.header.key);
        assert_eq!(&known_tips[..], &presumed_tips);
    }
}

fn make_test_user(
    secp: &Secp256k1<All>,
    handle: &db_handle::MsgDBHandle<'_>,
    name: String,
) -> KeyPair {
    let (kp, nonce, envelope) = generate_new_user(secp).unwrap();
    let u = handle.save_keypair(kp).unwrap();
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
#[tokio::test]
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
            "hidden_services",
            "message_nonces",
            "messages",
            "private_keys",
            "users"
        ],
        vit
    )
}