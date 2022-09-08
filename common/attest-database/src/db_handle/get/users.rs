use super::super::sql_serializers;
use crate::db_handle::{handle_type, MsgDBHandle};
use fallible_iterator::FallibleIterator;
use sapio_bitcoin;
use sapio_bitcoin::hashes::hex::ToHex;
use sapio_bitcoin::secp256k1::SecretKey;
use sapio_bitcoin::XOnlyPublicKey;
use std::collections::BTreeMap;

const SQL_GET_ALL_USERS: &'static str = include_str!("../sql/get/users/all_users.sql");
const SQL_GET_USER_BY_KEY: &'static str = include_str!("../sql/get/users/user_by_key.sql");
const SQL_GET_ALL_SECRET_KEYS: &'static str = include_str!("../sql/get/users/all_secret_keys.sql");

impl<'a, T> MsgDBHandle<'a, T>
where
    T: handle_type::Get,
{
    /// finds a user by key
    pub fn locate_user(
        &self,
        key: &sapio_bitcoin::secp256k1::XOnlyPublicKey,
    ) -> Result<(i64, String), rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_USER_BY_KEY)?;
        stmt.query_row([key.to_hex()], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
    }

    /// build a keymap for all known keypairs.
    pub fn get_keymap(&self) -> Result<BTreeMap<XOnlyPublicKey, SecretKey>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_ALL_SECRET_KEYS)?;
        let rows = stmt.query([])?;
        rows.map(|r| {
            Ok((
                r.get::<_, sql_serializers::PK>(0)?.0,
                r.get::<_, sql_serializers::SK>(1)?.0,
            ))
        })
        .collect()
    }

    pub fn get_all_users(&self) -> Result<Vec<(XOnlyPublicKey, String)>, rusqlite::Error> {
        let mut stmt = self.0.prepare_cached(SQL_GET_ALL_USERS)?;
        let q = stmt.query([])?;

        q.mapped(|row| {
            let xonly_public_key = row.get::<_, sql_serializers::PK>(0)?.0;
            let nickname = row.get::<_, String>(1)?;
            Ok((xonly_public_key, nickname))
        })
        .collect()
    }
}
