use std::{collections::BTreeMap, num::NonZeroU128, ops::Div};

use crate::{callbacks::Callback, entity::EntityID, game::GameBoard};

/// PowerPlantEvent drives the event loop for powerplants, including e.g.
/// distribution of mining rewards.
#[derive(Clone, Debug)]
pub struct PowerPlantEvent {
    pub time: u64,
    pub period: u64,
}
impl Callback for PowerPlantEvent {
    fn time(&self) -> u64 {
        self.time
    }

    fn action(&mut self, game: &mut GameBoard) {
        let plants = game.nfts.power_plants.clone();
        let mut total = 0;
        let mut shares: BTreeMap<EntityID, u128> = BTreeMap::new();
        for (id, plant) in plants {
            let share = plant.compute_hashrate(game);
            total += share;
            let owner = game.nfts[id].owner();
            *shares.entry(owner).or_default() += share;
        }
        if let Some(total) = NonZeroU128::new(total) {
            shares
                .values_mut()
                .for_each(|v| *v = ((*v * 1024 * game.mining_subsidy).div(total)) / 1024);

            let btc = &mut game.tokens[game.bitcoin_token_id];
            btc.transaction();
            for (to, amount) in shares {
                btc.mint(&to, amount)
            }
            btc.end_transaction();
        }

        // Reschedule
        self.time = game.elapsed_time + self.period;
        game.callbacks.schedule(Box::new(self.clone()));
    }

    fn purpose(&self) -> String {
        "Periodic Mining Payout Delivery".into()
    }
}
