// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ruma_serde::CanonicalJsonValue;
use rusqlite::types::FromSql;
use rusqlite::types::FromSqlError;
use rusqlite::types::ToSqlOutput;
use rusqlite::ToSql;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::secp256k1::SecretKey;
use sapio_bitcoin::XOnlyPublicKey;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::convert::TryFrom;
use std::str::FromStr;

pub(crate) struct SK(pub SecretKey);

pub(crate) struct PK(pub XOnlyPublicKey);
impl ToSql for PK {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.0.to_hex().into())
    }
}

impl ToSql for SK {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.0.secret_bytes().to_hex().into())
    }
}

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

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct SqlJson(pub CanonicalJsonValue);
#[derive(Serialize)]
#[serde(transparent)]
pub struct SqlJsonRef<'a>(pub &'a CanonicalJsonValue);

impl ToSql for SqlJsonRef<'_> {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.to_string()))
    }
}
impl ToSql for SqlJson {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.to_string()))
    }
}
impl FromSql for SqlJson {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let json: Value =
            serde_json::from_str(value.as_str()?).map_err(|e| FromSqlError::Other(e.into()))?;
        CanonicalJsonValue::try_from(json)
            .map_err(|e| FromSqlError::Other(e.into()))
            .map(SqlJson)
    }
}
