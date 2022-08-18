use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    erc20::ERC20Registry,
    game::GameBoard,
    nft::{NFTRegistry, NFT},
};

#[derive(Default)]
pub struct CallbackRegistry {
    callbacks: BTreeMap<u64, Vec<Box<dyn Callback>>>,
}

impl CallbackRegistry {
    pub(crate) fn schedule(&mut self, cb: Box<dyn Callback>) {
        let v = self.callbacks.entry(cb.time()).or_default();
        v.push(cb);
    }
}

impl Serialize for CallbackRegistry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.callbacks.iter().map(
            |(k, v): (&u64, &Vec<Box<dyn Callback>>)| {
                (k, v.iter().map(|x| x.purpose()).collect::<Vec<String>>())
            },
        ))
    }
}
pub(crate) trait Callback: Send + Sync {
    fn time(&self) -> u64;
    fn action(&mut self, game: &mut GameBoard);
    fn purpose(&self) -> String;
}

impl CallbackRegistry {
    fn run(game: &mut GameBoard) {
        let s = &mut game.callbacks;
        // get everything that is to_do in the future and remove it...
        let mut to_do = s.callbacks.split_off(&(game.current_time + 1));
        // swap it with the things in the present
        std::mem::swap(&mut s.callbacks, &mut to_do);
        for (k, v) in to_do {
            for mut x in v.into_iter() {
                x.action(game);
            }
        }
    }
}
