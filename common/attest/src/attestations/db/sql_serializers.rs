use crate::attestations::messages::Envelope;
use crate::attestations::nonce::PrecomittedNonce;
use crate::attestations::nonce::PrecomittedPublicNonce;
use ruma_serde::CanonicalJsonValue;
use rusqlite::types::FromSql;
use rusqlite::types::FromSqlError;
use rusqlite::ToSql;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::secp256k1::SecretKey;
use sapio_bitcoin::XOnlyPublicKey;
use std::collections::BTreeMap;
use std::str::FromStr;

pub(crate) struct SK(pub SecretKey);

pub(crate) struct PK(pub XOnlyPublicKey);

impl FromSql for SK {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        SecretKey::from_str(s)
            .map_err(|e| FromSqlError::Other(Box::new(e)))
            .map(SK)
    }
}

impl FromSql for PK {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        XOnlyPublicKey::from_str(s)
            .map_err(|e| FromSqlError::Other(Box::new(e)))
            .map(PK)
    }
}

impl ToSql for Envelope {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let s = serde_json::to_value(self)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let c: BTreeMap<String, CanonicalJsonValue> = serde_json::from_value(s)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ruma_signatures::canonical_json(&c)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
            .into())
    }
}
impl FromSql for Envelope {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        serde_json::from_str(s).map_err(|e| rusqlite::types::FromSqlError::Other(e.into()))
    }
}

impl ToSql for PrecomittedNonce {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.0.secret_bytes().to_hex().into())
    }
}
impl FromSql for PrecomittedNonce {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        SecretKey::from_str(value.as_str()?)
            .map(PrecomittedNonce)
            .map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}

impl ToSql for PrecomittedPublicNonce {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.0.to_hex().into())
    }
}
impl FromSql for PrecomittedPublicNonce {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        XOnlyPublicKey::from_str(value.as_str()?)
            .map(PrecomittedPublicNonce)
            .map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}
