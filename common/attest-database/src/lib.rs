use std::{error::Error, path::PathBuf, sync::Arc};

use connection::MsgDB;
use rusqlite::Connection;

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
