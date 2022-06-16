use super::connection::MsgDB;
use super::*;
use crate::attestations::messages::{Envelope, Header, InnerMessage, Unsigned};
use crate::attestations::nonce::PrecomittedNonce;
use crate::util;
use fallible_iterator::FallibleIterator;
use rusqlite::{params, Connection};
use sapio_bitcoin::hashes::{sha256, Hash};
use sapio_bitcoin::secp256k1::{rand, All, Secp256k1};
use sapio_bitcoin::KeyPair;
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
async fn test_envelope_creation() {
    let conn = setup_db().await;
    let secp = Secp256k1::new();
    let test_user = "TestUser".into();
    let handle = conn.get_handle().await;
    let kp = make_test_user(&secp, &handle, test_user);
    let envelope_1 = handle
        .wrap_message_in_envelope_for_user_by_key(InnerMessage::Ping(10), &kp, &secp)
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
    {
        let known_tips = handle.get_tip_for_known_keys().unwrap();
        assert_eq!(known_tips.len(), 1);
        assert_eq!(&known_tips[0], envelope_1.inner_ref());
    }

    let envelope_2 = handle
        .wrap_message_in_envelope_for_user_by_key(InnerMessage::Ping(10), &kp, &secp)
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

    let kp_2 = make_test_user(&secp, &handle, "TestUser2".into());

    let envelope_3 = handle
        .wrap_message_in_envelope_for_user_by_key(InnerMessage::Ping(10), &kp_2, &secp)
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
            envelope_2.inner_ref().clone(),
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
    let mut rng = rand::thread_rng();
    let (sk, pk) = secp.generate_keypair(&mut rng);
    let key = pk.x_only_public_key().0;
    let nonce = PrecomittedNonce::new(secp);
    let kp = KeyPair::from_secret_key(secp, &sk);
    handle.save_keypair(kp).unwrap();
    let mut genesis = Envelope {
        header: Header {
            key,
            next_nonce: nonce.get_public(secp),
            prev_msg: sha256::Hash::hash(&[]),
            tips: vec![],
            height: 0,
            sent_time_ms: util::now(),
            unsigned: Unsigned { signature: None },
        },
        msg: InnerMessage::Ping(0),
    };
    genesis
        .sign_with(&kp, secp, PrecomittedNonce::new(secp))
        .unwrap();
    let genesis = genesis.self_authenticate(secp).unwrap();
    handle
        .insert_user_by_genesis_envelope(name, genesis)
        .unwrap();
    handle.save_nonce_for_user_by_key(nonce, secp, key).unwrap();
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
