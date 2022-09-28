//! A system for scheduling and running events asynchronously
use crate::game::GameBoard;
use serde::Serialize;
use std::collections::BTreeMap;

/// The registry of events. Events are processed in linear time order, then
/// secondarily the order they are recieved
#[derive(Default)]
pub struct CallbackRegistry {
    /// the key in this type is a virtual "time" at which the event should be
    /// removed and processed
    callbacks: BTreeMap<u64, Vec<Box<dyn Callback>>>,
}

impl CallbackRegistry {
    /// Adds a Callback to the list;
    pub(crate) fn schedule(&mut self, cb: Box<dyn Callback>) {
        let v = self.callbacks.entry(cb.time()).or_default();
        v.push(cb);
    }
}

/// We implement Serialize for our Callbacks just for a human-readable representation
impl Serialize for CallbackRegistry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.callbacks.iter().map(
            |(k, v): (&u64, &Vec<Box<dyn Callback + 'static>>)| {
                (k, v.iter().map(|x| x.purpose()).collect::<Vec<String>>())
            },
        ))
    }
}
/// Callback must be implemented in order to register a future event
pub(crate) trait Callback: Send + Sync {
    /// When the event should be fired, may be fired later than requested
    fn time(&self) -> u64;
    /// Run the callback. Has access to entire GameBoard
    fn action(&mut self, game: &mut GameBoard);
    /// A shorthand convenience for debugging
    fn purpose(&self) -> String;
}

impl CallbackRegistry {
    /// run drains the event queue of all events happening at or before the current time, and processes them all
    pub fn run(game: &mut GameBoard) {
        let s = &mut game.callbacks;
        // get everything that is to_do in the future and remove it...
        let mut to_do = s.callbacks.split_off(&(game.elapsed_time + 1));
        // swap it with the things in the present
        std::mem::swap(&mut s.callbacks, &mut to_do);
        for (_k, v) in to_do {
            for mut x in v.into_iter() {
                x.action(game);
            }
        }
    }
}
