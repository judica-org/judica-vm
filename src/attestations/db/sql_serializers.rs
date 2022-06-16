use std::str::FromStr;

use rusqlite::types::FromSqlError;
use rusqlite::Connection;

use rusqlite::types;

use rusqlite::types::FromSql;

use sapio_bitcoin::PublicKey;

use sapio_bitcoin::secp256k1::SecretKey;
use sapio_bitcoin::XOnlyPublicKey;

pub(crate) struct SK(pub SecretKey);

pub(crate) struct PK(pub XOnlyPublicKey);

impl FromSql for SK {
    fn column_result(
        value: rusqlite::types::ValueRef<'_>,
    ) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        SecretKey::from_str(s)
            .map_err(|e| FromSqlError::Other(Box::new(e)))
            .map(SK)
    }
}

impl FromSql for PK {
    fn column_result(
        value: rusqlite::types::ValueRef<'_>,
    ) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        XOnlyPublicKey::from_str(s)
            .map_err(|e| FromSqlError::Other(Box::new(e)))
            .map(PK)
    }
}
