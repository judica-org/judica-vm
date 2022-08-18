use serde::Serialize;

use crate::{callbacks::Callback, game::GameBoard, nfts::NftPtr, tokens::TokenPointer};

#[derive(Serialize, Clone)]
pub(crate) struct CoinLockup {
    pub id: NftPtr,
    pub time_when_free: u64,
    pub asset: TokenPointer,
}

impl Callback for CoinLockup {
    fn time(&self) -> u64 {
        self.time_when_free
    }

    // Note: just reads immutable fields, modifies external state
    fn action(&mut self, game: &mut GameBoard) {
        let owner = game.nfts[self.id].owner();
        if game.current_time < self.time_when_free {
            return;
        }
        let token = &mut game.tokens[self.asset];
        token.transaction();
        let balance = token.balance_check(&self.id.0);
        let _ = token.transfer(&self.id.0, &owner, balance);
        token.end_transaction();
    }

    fn purpose(&self) -> String {
        format!("CoinLockup Release Trigger")
    }
}
