use super::TokenBase;
use super::TokenPointer;
use crate::entity::EntityID;
use crate::game::CallContext;
use crate::game::GameBoard;
use crate::tokens::TokenRegistry;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fmt::Display;
use tracing::Instrument;

/// Data for a single trading pair (e.g. Apples to Oranges tokens)
///
/// Pairs have a balance in Apples and Oranges, as well as a token that represents
/// a fractional interest (unit / total) redemptive right of Apples : Oranges
#[derive(Serialize)]
pub(crate) struct ConstantFunctionMarketMakerPair {
    /// The trading pair, should be normalized here
    pub(crate) pair: TradingPairID,
    /// The ID of this pair
    pub(crate) id: EntityID,
    /// The ID of the LP Tokens for this pair
    pub(crate) lp: TokenPointer,
}

impl ConstantFunctionMarketMakerPair {
    pub fn has_market(game: &GameBoard, mut pair: TradingPairID) -> bool {
        pair.normalize();
        game.swap.markets.contains_key(&pair)
    }
    /// ensure makes sure that a given trading pair exists in the GameBoard
    fn ensure(game: &mut GameBoard, mut pair: TradingPairID) -> TradingPairID {
        pair.normalize();
        match game.swap.markets.entry(pair) {
            std::collections::btree_map::Entry::Vacant(_a) => {
                let name_a = game.tokens[pair.asset_a]
                    .nickname()
                    .unwrap_or(format!("{}", pair.asset_a.inner()));
                let name_b = game.tokens[pair.asset_b]
                    .nickname()
                    .unwrap_or(format!("{}", pair.asset_b.inner()));
                let base_id = game.alloc();
                let id = game.alloc();
                game.swap.markets.insert(
                    pair,
                    ConstantFunctionMarketMakerPair {
                        pair,
                        id,
                        lp: game.tokens.new_token(Box::new(TokenBase {
                            balances: Default::default(),
                            total: Default::default(),
                            this: base_id,
                            #[cfg(test)]
                            in_transaction: None,
                            nickname: Some(format!("swap({},{})::shares", name_a, name_b)),
                        })),
                    },
                );
                pair
            }
            std::collections::btree_map::Entry::Occupied(_a) => pair,
        }
    }
    fn amt_a(&self, tokens: &mut TokenRegistry) -> u128 {
        tokens[self.pair.asset_a].balance_check(&self.id)
    }
    fn amt_b(&self, tokens: &mut TokenRegistry) -> u128 {
        tokens[self.pair.asset_b].balance_check(&self.id)
    }
}

/// A TradingPair, not guaranteed to be normalized (which can lead to weird
/// bugs) Auto-canonicalizing is undesirable since a user might specify
/// elsewhere in corresponding order what their trade is.
#[derive(
    Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Serialize, Deserialize, JsonSchema, Debug,
)]
#[serde(into = "String", try_from = "String")]
pub struct TradingPairID {
    pub asset_a: TokenPointer,
    pub asset_b: TokenPointer,
}

#[derive(Debug)]
pub enum TradingPairIDParseError {
    WrongNumberOfTerms,
    EntityIDParseError(<EntityID as TryFrom<String>>::Error),
}

impl Display for TradingPairIDParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}
impl std::error::Error for TradingPairIDParseError {}

impl TryFrom<String> for TradingPairID {
    type Error = TradingPairIDParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let a: Vec<&str> = value.split(':').collect();
        if a.len() != 2 {
            return Err(TradingPairIDParseError::WrongNumberOfTerms);
        }
        let asset_a = TokenPointer(
            EntityID::try_from(a[0]).map_err(TradingPairIDParseError::EntityIDParseError)?,
        );
        let asset_b = TokenPointer(
            EntityID::try_from(a[1]).map_err(TradingPairIDParseError::EntityIDParseError)?,
        );
        Ok(TradingPairID { asset_a, asset_b })
    }
}
impl From<TradingPairID> for String {
    fn from(s: TradingPairID) -> Self {
        format!(
            "{}:{}",
            String::from(s.asset_a.0),
            String::from(s.asset_b.0)
        )
    }
}

impl TradingPairID {
    /// Sort the key for use in e.g. Maps
    /// N.B. don't normalize without sorting the amounts in the same order.
    /// TODO: Make a type-safer way of represnting this
    pub fn normalize(&mut self) {
        if self.asset_a <= self.asset_b {
        } else {
            *self = Self {
                asset_a: self.asset_b,
                asset_b: self.asset_a,
            }
        }
    }
    fn is_normal(&self) -> bool {
        self.asset_a <= self.asset_b
    }
}

/// Registry of all Market Pairs
#[derive(Serialize, Default)]
pub(crate) struct ConstantFunctionMarketMaker {
    pub(crate) markets: BTreeMap<TradingPairID, ConstantFunctionMarketMakerPair>,
}

#[derive(Debug, Serialize, Clone)]
pub enum TradeError {
    InvalidTrade(String),
    InsufficientTokens(String),
    MarketSlipped,
}

impl Display for TradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradeError::InvalidTrade(msg) => write!(f, "Error: Invalid trade. {:?}", msg),
            TradeError::InsufficientTokens(msg) => {
                write!(f, "Error: Insufficient tokens to complete trade.{:?}", msg)
            }
            TradeError::MarketSlipped => write!(f, "Error: Market Slipped"),
        }
    }
}

impl ConstantFunctionMarketMaker {
    // TODO: Better math in this whole module

    /// Adds amount_a and amount_b to the pair
    /// N.B. The deposit will fail if amount_a : amount_b != mkt.amt_a() : mkt.amt_b()
    /// We might need some slack in how that works? May also be convenient to
    /// imply one param from the other...
    ///
    /// mints the corresponding # of LP tokens
    pub(crate) fn deposit(
        game: &mut GameBoard,
        mut id: TradingPairID,
        mut amount_a: u128,
        mut amount_b: u128,
        from: EntityID,
    ) {
        let unnormalized_id = id;
        id.normalize();
        if id != unnormalized_id {
            std::mem::swap(&mut amount_a, &mut amount_b);
        }
        if amount_a == 0 || amount_b == 0 {
            return;
        }
        let id = ConstantFunctionMarketMakerPair::ensure(game, id);
        let mkt = &game.swap.markets[&id];

        let tokens: &mut TokenRegistry = &mut game.tokens;
        tokens[id.asset_a].transaction();
        tokens[id.asset_b].transaction();

        //        amount_a / amount_b = mkt.amt_a / mkt.amt_b
        if amount_a * mkt.amt_b(tokens) != mkt.amt_a(tokens) * amount_b {
            // todo: does the above need slack?
            return;
        }

        if !tokens[id.asset_a].transfer(&from, &mkt.id, amount_a) {
            return;
        }

        if !tokens[id.asset_b].transfer(&from, &mkt.id, amount_b) {
            // attempt to return asset a if asset b transfer fails...
            // if the return transfer fails then??
            let _ = tokens[id.asset_a].transfer(&mkt.id, &from, amount_a);
            return;
        }

        let coins = tokens[mkt.lp].total_coins();

        let to_mint = (coins * amount_a) / mkt.amt_a(tokens);

        let lp_tokens = &mut tokens[mkt.lp];
        lp_tokens.transaction();
        lp_tokens.mint(&from, to_mint);
        lp_tokens.end_transaction();
        tokens[id.asset_a].end_transaction();
        tokens[id.asset_b].end_transaction();
    }

    /// TODO: Implement me please!
    ///
    /// This function should invert a deposit
    pub(crate) fn withdraw(&mut self) {
        todo!();
    }
    /// Perform a trade op by using the X*Y = K formula for a CFMM
    ///
    /// Parameters: One of amount_a or amount_b should be 0, which implies the trade direction
    pub(crate) fn do_sell_trade(
        game: &mut GameBoard,
        mut id: TradingPairID,
        mut amount_a: u128,
        mut amount_b: u128,
        buy_min: Option<u128>,
        simulate: bool,
        ctx: &CallContext,
    ) -> Result<TradeOutcome, TradeError> {
        let unnormalized_id = id;
        id.normalize();
        if id != unnormalized_id {
            std::mem::swap(&mut amount_a, &mut amount_b);
        }
        // the zero is the one to be computed
        if !(amount_a == 0 || amount_b == 0) {
            return Err(TradeError::InvalidTrade(
                "Both token amounts cannot be zero".into(),
            ));
        }
        ConstantFunctionMarketMaker::internal_do_trade_sell_fixed_amount(
            game, id, amount_a, amount_b, buy_min, simulate, ctx,
        )
    }

    pub(crate) fn do_buy_trade(
        game: &mut GameBoard,
        mut id: TradingPairID,
        mut amount_a: u128,
        mut amount_b: u128,
        sell_max: Option<u128>,
        simulate: bool,
        ctx: &CallContext,
    ) -> Result<TradeOutcome, TradeError> {
        let unnormalized_id = id;
        id.normalize();
        if id != unnormalized_id {
            std::mem::swap(&mut amount_a, &mut amount_b);
        }
        // the zero is the one to be computed
        if !(amount_a == 0 || amount_b == 0) {
            return Err(TradeError::InvalidTrade(
                "Both token amounts cannot be zero".into(),
            ));
        }
        ConstantFunctionMarketMaker::internal_do_trade_buy_fixed_amount(
            game, id, amount_a, amount_b, sell_max, simulate, ctx,
        )
    }

    pub(crate) fn internal_do_trade_buy_fixed_amount(
        game: &mut GameBoard,
        mut id: TradingPairID,
        mut amount_a: u128,
        mut amount_b: u128,
        // upper bound on amount to sell
        sell_max: Option<u128>,
        simulate: bool,
        CallContext { ref sender }: &CallContext,
    ) -> Result<TradeOutcome, TradeError> {
        if !id.is_normal() {
            panic!("Expects normalized IDs")
        }
        let id = ConstantFunctionMarketMakerPair::ensure(game, id);
        let mkt = &game.swap.markets[&id];
        let tokens: &mut TokenRegistry = &mut game.tokens;
        let (buying, selling, buy_amt) = match (amount_a, amount_b) {
            (0, b) => (id.asset_b, id.asset_a, b),
            (a, 0) => (id.asset_a, id.asset_b, a),
            _ => panic!("Expected one to be 0"),
        };

        let (
            asset_player_purchased,
            amount_player_purchased,
            asset_player_sold,
            amount_player_sold,
        ) = {
            let selling_asset_name = tokens[selling].nickname().unwrap();
            let buying_asset_name = tokens[buying].nickname().unwrap();
            // if a is zero, a is token being "purchased"
            // otherwise b is token being "purchased"
            //
            // mkt_qty_selling * y = k
            // (mkt_qty_selling + sell_amt) * (mkt_qty_buying - buy_amt) = k
            // (mkt_qty_selling + sell_amt) * (mkt_qty_buying - buy_amt) = (mkt_qty_selling * mkt_qty_buying)
            // (mkt_qty_selling*mkt_qty_buying)/(mkt_qty_buying-buy_amt) - mkt_qty_selling = sell_amt
            let mkt_qty_selling = tokens[selling].balance_check(&mkt.id);
            let mkt_qty_buying = tokens[buying].balance_check(&mkt.id);
            if buy_amt > mkt_qty_buying {
                return Err(TradeError::InsufficientTokens(
                    "Market has insufficient tokens".into(),
                ));
            }
            let k = mkt_qty_selling * mkt_qty_buying;
            let sell_amt = (k / (mkt_qty_buying - buy_amt)) - mkt_qty_selling;

            if let Some(max) = sell_max {
                if max > sell_amt {
                    return Err(TradeError::MarketSlipped);
                }
            }

            if !simulate {
                // Only check this condition when not simulating... as we're
                // looking to find out how much we would need!
                if sell_amt > tokens[selling].balance_check(sender) {
                    return Err(TradeError::InsufficientTokens(
                        "User has insufficient tokens".into(),
                    ));
                }
                tokens[buying].transaction();
                tokens[selling].transaction();
                let _ = tokens[selling].transfer(sender, &mkt.id, sell_amt);
                let _ = tokens[buying].transfer(&mkt.id, sender, buy_amt);
                tokens[buying].end_transaction();
                tokens[selling].end_transaction();
            }
            (buying_asset_name, buy_amt, selling_asset_name, sell_amt)
        };

        Ok(TradeOutcome {
            trading_pair: id,
            asset_player_purchased,
            amount_player_purchased,
            asset_player_sold,
            amount_player_sold,
        })
    }
    pub(crate) fn internal_do_trade_sell_fixed_amount(
        game: &mut GameBoard,
        mut id: TradingPairID,
        mut amount_a: u128,
        mut amount_b: u128,
        buy_min: Option<u128>,
        simulate: bool,
        CallContext { ref sender }: &CallContext,
    ) -> Result<TradeOutcome, TradeError> {
        if !id.is_normal() {
            panic!("Expects normalized IDs")
        }
        let id = ConstantFunctionMarketMakerPair::ensure(game, id);
        let mkt = &game.swap.markets[&id];
        let tokens: &mut TokenRegistry = &mut game.tokens;
        if !(tokens[id.asset_a].balance_check(sender) >= amount_a
            && tokens[id.asset_b].balance_check(sender) >= amount_b)
        {
            return Err(TradeError::InsufficientTokens(
                "Sender has insufficient tokens".into(),
            ));
        }

        if !(amount_a <= mkt.amt_a(tokens) && amount_b <= mkt.amt_b(tokens)) {
            return Err(TradeError::InsufficientTokens(
                "Market has insufficient tokens".into(),
            ));
        }

        let (buying, selling, sell_amt) = match (amount_a, amount_b) {
            (0, b) => (id.asset_a, id.asset_b, b),
            (a, 0) => (id.asset_b, id.asset_a, a),
            _ => panic!("Expected one to be 0"),
        };

        let (
            asset_player_purchased,
            amount_player_purchased,
            asset_player_sold,
            amount_player_sold,
        ) = {
            let asset_buying_name = tokens[buying].nickname().unwrap();
            let asset_selling_name = tokens[selling].nickname().unwrap();
            // if a is zero, a is token being "purchased"
            // the amount player receives of a

            let mkt_qty_selling = tokens[selling].balance_check(&mkt.id);
            let mkt_qty_buying = tokens[buying].balance_check(&mkt.id);

            if let Some(min) = buy_min {
                if mkt_qty_buying < min {
                    return Err(TradeError::InsufficientTokens(
                        "Not Enough Liquidity to Buy Min".into(),
                    ));
                }
            }
            let k = mkt_qty_buying * mkt_qty_selling;

            // mkt_qty_selling * mkt_qty_buying = k
            // mkt_qty_selling + sell_amt * mkt_qty_buying - buy_amt = k
            // mkt_qty_selling + sell_amt * mkt_qty_buying - buy_amt = mkt_qty_selling * mkt_qty_buying
            // mkt_qty_buying - buy_amt = (mkt_qty_selling * mkt_qty_buying) / (mkt_qty_selling + sell_amt)
            // - buy_amt = (mkt_qty_selling * mkt_qty_buying) / (mkt_qty_selling + sell_amt) - mkt_qty_buying
            // buy_amt = mkt_qty_buying - (mkt_qty_selling * mkt_qty_buying) / (mkt_qty_selling + sell_amt)
            let bought_amount = (mkt.amt_a(tokens) * amount_b) / mkt.amt_b(tokens);
            let buy_amt = mkt_qty_buying - (k / (mkt_qty_selling + sell_amt));
            if let Some(min) = buy_min {
                if bought_amount < min {
                    return Err(TradeError::MarketSlipped);
                }
            }
            if !simulate {
                tokens[buying].transaction();
                tokens[selling].transaction();
                let _ = tokens[selling].transfer(sender, &mkt.id, sell_amt);
                let _ = tokens[buying].transfer(&mkt.id, sender, buy_amt);
                tokens[buying].transaction();
                tokens[selling].transaction();
            }
            (asset_buying_name, buy_amt, asset_selling_name, sell_amt)
        };

        Ok(TradeOutcome {
            trading_pair: id,
            asset_player_purchased,
            amount_player_purchased,
            asset_player_sold,
            amount_player_sold,
        })
    }

    pub(crate) fn get_pair_price_data(game: &mut GameBoard, id: TradingPairID) -> (u128, u128) {
        // check that a these two tokens are a valid pairing (do we need to?)
        let id = ConstantFunctionMarketMakerPair::ensure(game, id);
        let tokens: &mut TokenRegistry = &mut game.tokens;
        // get the CFMM pair
        let mkt = &game.swap.markets[&id];
        let mkt_qty_a = mkt.amt_a(tokens);
        let mkt_qty_b = mkt.amt_b(tokens);

        (mkt_qty_a, mkt_qty_b)
    }
}

/// A struct for passing token qty information to the UX for price calculation
#[derive(Serialize, Clone, Debug)]
pub struct UXMaterialsPriceData {
    pub trading_pair: TradingPairID,
    pub asset_a: String,
    pub mkt_qty_a: u128,
    pub asset_b: String,
    pub mkt_qty_b: u128,
    pub display_asset: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct TradeOutcome {
    pub trading_pair: TradingPairID,
    pub asset_player_purchased: String,
    pub amount_player_purchased: u128,
    pub asset_player_sold: String,
    pub amount_player_sold: u128,
}
