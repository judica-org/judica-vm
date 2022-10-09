use attest_messages::{
    nonce::PrecomittedNonce, AttestEnvelopable, GenericEnvelope, Header, Unsigned,
};
use attest_util::{ensure_dir, CrossPlatformPermissions};
use connection::MsgDB;
use rusqlite::Connection;
use sapio_bitcoin::{
    secp256k1::{rand, Secp256k1, Signing},
    KeyPair,
};
use std::{error::Error, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use tracing::trace;

pub mod connection;
pub mod db_handle;
pub mod sql_error;
pub mod sql_serializers;

#[cfg(test)]
mod tests;

pub async fn setup_db_at(dir: PathBuf, name: &str) -> Result<MsgDB, Box<dyn Error>> {
    tracing::debug!(
        "Request to Open Message DB at {} name {}",
        dir.display(),
        name
    );
    let dir: PathBuf =
        ensure_dir(dir, CrossPlatformPermissions::unix_only_permissions(0o700)).await?;
    let mut db_file = dir.clone();
    db_file.push(name);
    db_file.set_extension("sqlite3");
    tracing::debug!("Opening Message DB at: {}", db_file.display());
    let conn = || Connection::open(&db_file).unwrap();
    let mdb = MsgDB::new(
        (0..10)
            .map(|_| Arc::new(tokio::sync::Mutex::new(conn())))
            .collect(),
    );
    mdb.map_all_sequential(|mut h| Box::pin(async move { h.setup_tables() }))
        .await;
    Ok(mdb)
}
pub async fn setup_db(application: &str, prefix: Option<PathBuf>) -> Result<MsgDB, Box<dyn Error>> {
    let dirs = directories::ProjectDirs::from("org", "judica", application).unwrap();
    let data_dir: PathBuf = dirs.data_dir().into();
    let data_dir = if let Some(prefix) = prefix {
        tracing::debug!("Creating DB with Prefix {}", prefix.display());
        prefix.join(data_dir.strip_prefix("/")?)
    } else {
        data_dir
    };
    setup_db_at(data_dir, "attestations").await
}

pub async fn setup_test_db() -> MsgDB {
    trace!("Setting up Test DB");
    let first = Arc::new(Mutex::new(Connection::open(":memory:").unwrap()));
    let conn = MsgDB::new(vec![
        first.clone(),
        first.clone(),
        first.clone(),
        first.clone(),
    ]);
    conn.map_all_sequential(|mut h| Box::pin(async move { h.setup_tables() }))
        .await;
    conn
}
pub fn generate_new_user<C: Signing, M: AttestEnvelopable, Im: Into<M>>(
    secp: &Secp256k1<C>,
    init: Im,
) -> Result<(KeyPair, PrecomittedNonce, GenericEnvelope<M>), Box<dyn Error>> {
    let keypair: _ = KeyPair::new(secp, &mut rand::thread_rng());
    let nonce = PrecomittedNonce::new(secp);
    let next_nonce = PrecomittedNonce::new(secp);
    let sent_time_ms = attest_util::now();

    let pub_next_nonce = next_nonce.get_public(secp);
    let mut msg = GenericEnvelope::new(
        Header::new(
            keypair.public_key().x_only_public_key().0,
            pub_next_nonce,
            None,
            vec![],
            0,
            sent_time_ms,
            Unsigned::new(Default::default()),
            Default::default(),
        ),
        init,
    );
    msg.sign_with(&keypair, secp, nonce)?;
    Ok((keypair, next_nonce, msg))
}
