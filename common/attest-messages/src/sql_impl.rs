use crate::nonce::{PrecomittedNonce, PrecomittedPublicNonce};
use crate::{CanonicalEnvelopeHash, Envelope};
use rusqlite::types::{FromSql, FromSqlError};
use rusqlite::ToSql;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::hashes::sha256;
use sapio_bitcoin::secp256k1::SecretKey;
use sapio_bitcoin::XOnlyPublicKey;
use std::collections::BTreeMap;
use std::str::FromStr;
impl ToSql for Envelope {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let s = serde_json::to_value(self)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let c: BTreeMap<String, ruma_serde::CanonicalJsonValue> = serde_json::from_value(s)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ruma_serde::to_canonical_value(&c)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
            .to_string()
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

// Implemented here to keep type opaque
impl ToSql for CanonicalEnvelopeHash {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(self.0.to_hex().into())
    }
}
impl FromSql for CanonicalEnvelopeHash {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        sha256::Hash::from_str(value.as_str()?)
            .map(CanonicalEnvelopeHash)
            .map_err(|e| FromSqlError::Other(Box::new(e)))
    }
}
