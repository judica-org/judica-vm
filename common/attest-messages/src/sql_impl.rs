// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::nonce::{PrecomittedNonce, PrecomittedPublicNonce};
use crate::{AttestEnvelopable, Authenticated, CanonicalEnvelopeHash, GenericEnvelope};
use rusqlite::types::{FromSql, FromSqlError, ToSqlOutput};
use rusqlite::ToSql;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::hashes::sha256;
use sapio_bitcoin::secp256k1::SecretKey;
use sapio_bitcoin::XOnlyPublicKey;
use std::str::FromStr;
impl<T> ToSql for GenericEnvelope<T>
where
    T: AttestEnvelopable,
{
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let cv = ruma_serde::to_canonical_value(self)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ToSqlOutput::from(cv.to_string()))
    }
}
impl<T> FromSql for GenericEnvelope<T>
where
    T: AttestEnvelopable,
{
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        serde_json::from_str(s).map_err(|e| rusqlite::types::FromSqlError::Other(e.into()))
    }
}

impl<T> FromSql for Authenticated<GenericEnvelope<T>>
where
    T: AttestEnvelopable,
{
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        let envelope: GenericEnvelope<T> =
            serde_json::from_str(s).map_err(|e| rusqlite::types::FromSqlError::Other(e.into()))?;
        Ok(Authenticated(envelope))
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
