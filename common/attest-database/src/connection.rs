// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::db_handle::MsgDBHandle;
use crate::db_handle::handle_type::{self, All};
use rusqlite::Connection;
use sapio_bitcoin::secp256k1::rand::{seq::SliceRandom, thread_rng};
use std::{marker::PhantomData, pin::Pin, sync::Arc};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct MsgDB(Arc<Vec<Arc<Mutex<Connection>>>>);

impl MsgDB {
    pub fn new(db: Vec<Arc<Mutex<Connection>>>) -> Self {
        if db.len() < 2 {
            panic!("Expected at least two connections, one read one write")
        }
        MsgDB(Arc::new(db))
    }

    pub async fn map_all_sequential<F>(&self, f: F)
    where
        F: Fn(MsgDBHandle<All>) -> Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
    {
        for conn in self.0.iter() {
            let h = MsgDBHandle(conn.clone().lock_owned().await, PhantomData::default());
            f(h).await;
        }
    }

    pub async fn get_handle_all(&self) -> MsgDBHandle<handle_type::All> {
        let conns = &self.0;
        tracing::trace!("Getting Write Handle to DB...");
        let first = conns[0].clone().lock_owned().await;
        tracing::trace!("Write Handle Acquired");
        MsgDBHandle(first, PhantomData::default())
    }

    pub async fn get_handle_read(&self) -> MsgDBHandle<handle_type::ReadOnly> {
        let conns = &self.0;
        tracing::trace!("Getting Read Handle to DB");
        // try N random locks
        for _lock in 1..conns.len() {
            let lock = SliceRandom::choose(&conns[1..], &mut thread_rng())
                .expect("conns known to be >= 2 in length");
            if let Ok(l) = lock.clone().try_lock_owned() {
                tracing::trace!("Read Handle Acquired");
                return MsgDBHandle(l, PhantomData::default());
            }
        }
        // pick a random lock to sleep on
        let l = SliceRandom::choose(&conns[1..], &mut thread_rng())
            .expect("conns known to be >= 2 in length")
            .clone();
        let l = l.lock_owned().await;
        tracing::trace!("Read Handle Acquired");
        MsgDBHandle(l, PhantomData::default())
    }
}
