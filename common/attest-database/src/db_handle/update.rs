use rusqlite::named_params;

use super::handle_type;
use super::MsgDBHandle;
impl<'a, T> MsgDBHandle<'a, T>
where
    T: handle_type::Get + handle_type::Insert,
{
    /// Normally not required, as triggered on DB insert
    pub fn resolve_parents(&mut self) -> Result<(), rusqlite::Error> {
        let txn = self.0.transaction()?;
        {
            let mut s = txn.prepare(include_str!("sql/update/resolve_prev_ids.sql"))?;
            loop {
                let mut modified = 1000;
                modified = s.execute(named_params! {":limit": modified})?;
                if modified == 0 {
                    break;
                }
            }
        }

        txn.commit()?;
        Ok(())
    }
    /// Required to run periodically to make progress...
    /// TODO: Something more efficient?
    pub fn attach_tips(&mut self) -> Result<(), rusqlite::Error> {
        let txn = self.0.transaction()?;
        {
            let mut s = txn.prepare(include_str!("sql/update/do_connect.sql"))?;
            loop {
                let mut modified = 1000;
                modified = s.execute(named_params! {":limit": modified})?;
                if modified == 0 {
                    break;
                }
            }
        }
        txn.commit()?;
        Ok(())
    }
}
