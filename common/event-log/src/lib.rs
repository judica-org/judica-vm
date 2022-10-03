use std::{error::Error, path::PathBuf, sync::Arc};

use attest_util::{ensure_dir, CrossPlatformPermissions};
use connection::EventLog;
use rusqlite::Connection;

pub mod connection;
pub mod db_handle;
pub mod sql_error;
pub mod sql_serializers;

#[cfg(test)]
mod tests;

pub async fn setup_db_at(dir: PathBuf, name: &str) -> Result<EventLog, Box<dyn Error>> {
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
    let conn = Connection::open(db_file).unwrap();
    let mdb = EventLog::new(Arc::new(tokio::sync::Mutex::new(conn)));
    mdb.get_accessor().await.setup_tables();
    Ok(mdb)
}
pub async fn setup_db(
    application: &str,
    prefix: Option<PathBuf>,
) -> Result<EventLog, Box<dyn Error>> {
    let dirs = directories::ProjectDirs::from("org", "judica", application).unwrap();
    let data_dir: PathBuf = dirs.data_dir().into();
    let data_dir = if let Some(prefix) = prefix {
        tracing::debug!("Creating DB with Prefix {}", prefix.display());
        prefix.join(data_dir.strip_prefix("/")?)
    } else {
        data_dir
    };
    setup_db_at(data_dir, "event_log").await
}
