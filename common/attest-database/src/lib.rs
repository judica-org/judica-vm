use std::{error::Error, path::PathBuf, sync::Arc};

use attest_messages::{nonce::PrecomittedNonce, CanonicalEnvelopeHash, Envelope, Header, Unsigned};
use connection::MsgDB;
use rusqlite::Connection;
use sapio_bitcoin::{
    secp256k1::{rand, Secp256k1, Signing},
    KeyPair,
};
use serde_json::Value;

pub mod connection;
pub mod db_handle;
pub mod sql_serializers;

#[cfg(test)]
mod tests;

pub async fn setup_db(application: &str) -> Result<MsgDB, Box<dyn Error>> {
    let dirs = directories::ProjectDirs::from("org", "judica", application).unwrap();
    let data_dir: PathBuf = ensure_dir(dirs.data_dir().into()).await?;
    let mut chat_db_file = data_dir.clone();
    chat_db_file.push("chat.sqlite3");
    let mdb = MsgDB::new(Arc::new(tokio::sync::Mutex::new(
        Connection::open(chat_db_file).unwrap(),
    )));
    mdb.get_handle().await.setup_tables();
    Ok(mdb)
}

async fn ensure_dir(data_dir: PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    let dir = tokio::fs::create_dir_all(&data_dir).await;
    match dir.as_ref().map_err(std::io::Error::kind) {
        Err(std::io::ErrorKind::AlreadyExists) => (),
        _e => dir?,
    };
    Ok(data_dir)
}

pub fn generate_new_user<C: Signing>(
    secp: &Secp256k1<C>,
) -> Result<(KeyPair, PrecomittedNonce, Envelope), Box<dyn Error>> {
    let keypair: _ = KeyPair::new(&secp, &mut rand::thread_rng());
    let nonce = PrecomittedNonce::new(&secp);
    let next_nonce = PrecomittedNonce::new(&secp);
    let sent_time_ms = attest_util::now();
    let mut msg = Envelope {
        header: Header {
            height: 0,
            prev_msg: CanonicalEnvelopeHash::genesis(),
            tips: Vec::new(),
            next_nonce: next_nonce.get_public(&secp),
            key: keypair.public_key().x_only_public_key().0,
            sent_time_ms,
            unsigned: Unsigned {
                signature: Default::default(),
            },
            checkpoints: Default::default(),
        },
        msg: Value::Null,
    };
    msg.sign_with(&keypair, &secp, nonce)?;
    Ok((keypair, next_nonce, msg))
}
