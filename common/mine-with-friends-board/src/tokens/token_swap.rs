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
use std::ops::Add;
use std::ops::Div;
use std::ops::Sub;
use tracing::trace;

/// Data for a single trading pair (e.g. Apples to Oranges tokens)
///
/// Pairs have a balance in Apples and Oranges, as well as a token that represents
/// a fractional interest (unit / total) redemptive right of Apples : Oranges
#[derive(Serialize, Clone, Copy, JsonSchema, Debug)]
pub(crate) struct ConstantFunctionMarketMakerPair {
    /// The trading pair, should be normalized here
    pub(crate) pair: TradingPairID,
    /// The ID of this pair
    pub(crate) id: EntityID,
    /// The ID of the LP Tokens for this pair
    pub(crate) lp: TokenPointer,
    pub(crate) reserve_a: u128,
    pub(crate) reserve_b: u128,
}

impl ConstantFunctionMarketMakerPair {
    pub fn has_market(game: &GameBoard, mut pair: TradingPairID) -> bool {
        pair.normalize();
        game.swap.markets.contains_key(&pair)
    }
    /// ensure makes sure that a given trading pair exists in the GameBoard
    pub fn ensure(
        game: &mut GameBoard,
        mut pair: TradingPairID,
    ) -> ConstantFunctionMarketMakerPair {
        pair.normalize();
        match game.swap.markets.entry(pair) {
            std::collections::btree_map::Entry::Vacant(v) => {
                let name_a = game.tokens[pair.asset_a]
                    .nickname()
                    .unwrap_or(format!("{}", pair.asset_a.inner()));
                let name_b = game.tokens[pair.asset_b]
                    .nickname()
                    .unwrap_or(format!("{}", pair.asset_b.inner()));
                // must take .alloc.make() to convince the borrow checker...
                let base_id = game.alloc.make();
                let id = game.alloc.make();
                *v.insert(ConstantFunctionMarketMakerPair {
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
                    reserve_a: 0,
                    reserve_b: 0,
                })
            }
            std::collections::btree_map::Entry::Occupied(a) => *a.get(),
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
#[derive(Eq, Ord, PartialEq, PartialOrd, Copy, Clone, Serialize, Deserialize, Debug)]
#[serde(into = "String", try_from = "String")]
pub struct TradingPairID {
    pub asset_a: TokenPointer,
    pub asset_b: TokenPointer,
}

impl JsonSchema for TradingPairID {
    fn schema_name() -> String {
        "TradingPairID".into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        String::json_schema(gen)
    }
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
#[derive(Serialize, Default, JsonSchema, Debug)]
pub(crate) struct ConstantFunctionMarketMaker {
    pub(crate) markets: BTreeMap<TradingPairID, ConstantFunctionMarketMakerPair>,
}

#[derive(Debug, Serialize, Clone, JsonSchema)]
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
        if !id.is_normal() {
            std::mem::swap(&mut amount_a, &mut amount_b);
            id.normalize();
        }
        if amount_a == 0 || amount_b == 0 {
            return;
        }
        let mkt = ConstantFunctionMarketMakerPair::ensure(game, id);

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

        let pair = game.swap.markets.get_mut(&mkt.pair).expect("Must exist...");
        pair.reserve_a += amount_a;
        pair.reserve_b += amount_b;
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
        id: TradingPairID,
        amount_a: u128,
        amount_b: u128,
        buy_min: Option<u128>,
        simulate: bool,
        ctx: &CallContext,
    ) -> Result<TradeOutcome, TradeError> {
        ConstantFunctionMarketMaker::internal_do_trade_sell_fixed_amount(
            game, id, amount_a, amount_b, buy_min, simulate, ctx,
        )
    }

    pub(crate) fn do_buy_trade(
        game: &mut GameBoard,
        id: TradingPairID,
        amount_a: u128,
        amount_b: u128,
        sell_max: Option<u128>,
        simulate: bool,
        ctx: &CallContext,
    ) -> Result<TradeOutcome, TradeError> {
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
            std::mem::swap(&mut amount_a, &mut amount_b);
            id.normalize();
        }
        let mkt = ConstantFunctionMarketMakerPair::ensure(game, id);
        let tokens: &mut TokenRegistry = &mut game.tokens;
        let (buying, selling, buy_amt) = match (amount_a, amount_b) {
            (0, 0) => {
                return Err(TradeError::InvalidTrade(
                    "Both token amounts cannot be zero".into(),
                ))
            }
            (0, b) => (id.asset_b, id.asset_a, b),
            (a, 0) => (id.asset_a, id.asset_b, a),
            _ => {
                return Err(TradeError::InvalidTrade(
                    "One token amount must be zero".into(),
                ))
            }
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
            let sell_amt = (k.roundup_div(mkt_qty_buying - buy_amt)) - mkt_qty_selling;
            let buy_amt = mkt_qty_buying - k.roundup_div(mkt_qty_selling + sell_amt);

            if let Some(max) = sell_max {
                if max > sell_amt {
                    return Err(TradeError::MarketSlipped);
                }
            }

            if sell_amt > tokens[selling].balance_check(sender) {
                return Err(TradeError::InsufficientTokens(
                    "User has insufficient tokens".into(),
                ));
            }
            if !simulate {
                tokens[selling].transaction();
                trace!(sell_amt, "Transferring Funds to Swap");
                if !tokens[selling].transfer(sender, &mkt.id, sell_amt) {
                    panic!("Logic Error: Invariant (Enough Balance) Already Checked")
                }
                tokens[selling].end_transaction();
                if let Err(e) = Self::swap_helper(selling, buying, game, 0, buy_amt, sender) {
                    let tokens = &mut game.tokens;
                    tokens[selling].transaction();
                    trace!(sell_amt, "Returning Funds Outer");
                    let _ = tokens[selling].transfer(&mkt.id, sender, sell_amt);
                    trace!("Funds Returned Outer");
                    tokens[selling].end_transaction();
                    return Err(e);
                }
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

    pub(crate) fn raw_swap(
        game: &mut GameBoard,
        mut id: TradingPairID,
        mut amount_a: u128,
        mut amount_b: u128,
        CallContext { ref sender }: &CallContext,
    ) -> Result<(), TradeError> {
        if !id.is_normal() {
            std::mem::swap(&mut amount_a, &mut amount_b);
            id.normalize();
        }
        let mkt = ConstantFunctionMarketMakerPair::ensure(game, id);

        let tokens: &mut TokenRegistry = &mut game.tokens;
        tokens[mkt.pair.asset_a].transaction();
        tokens[mkt.pair.asset_b].transaction();
        let send_a_amt =
            if amount_a > 0 && tokens[mkt.pair.asset_a].transfer(&mkt.id, sender, amount_a) {
                amount_a
            } else {
                0
            };
        let send_b_amt =
            if amount_b > 0 && tokens[mkt.pair.asset_b].transfer(&mkt.id, sender, amount_b) {
                amount_b
            } else {
                0
            };
        if !(send_a_amt == amount_a && send_b_amt == amount_b) {
            trace!("Returning funds");
            // undo swap
            if !tokens[mkt.pair.asset_a].transfer(sender, &mkt.id, amount_a) {
                panic!("Corrupt Game");
            }
            if !tokens[mkt.pair.asset_b].transfer(sender, &mkt.id, amount_b) {
                panic!("Corrupt Game");
            }
            trace!("Funds Returned");
            tokens[mkt.pair.asset_b].end_transaction();
            tokens[mkt.pair.asset_a].end_transaction();
            return Err(TradeError::InsufficientTokens(
                "Insufficient funds asset".to_string(),
            ));
        }
        tokens[mkt.pair.asset_b].end_transaction();
        tokens[mkt.pair.asset_a].end_transaction();

        let a_reserves_2 = tokens[mkt.pair.asset_a].balance_check(&mkt.id);
        let b_reserves_2 = tokens[mkt.pair.asset_b].balance_check(&mkt.id);
        let k_2 = a_reserves_2 * b_reserves_2;
        let k_1 = mkt.reserve_a * mkt.reserve_b;

        trace!(
            "Invariant: ({} * {} = {}) >= ({} = {} * {}) ?",
            a_reserves_2,
            b_reserves_2,
            k_2,
            k_1,
            mkt.reserve_a,
            mkt.reserve_b
        );
        if k_2 < k_1 {
            tokens[mkt.pair.asset_a].transaction();
            tokens[mkt.pair.asset_b].transaction();

            trace!("Returning funds");
            if !tokens[mkt.pair.asset_a].transfer(sender, &mkt.id, amount_a) {
                panic!("Corrupt Game");
            }
            if !tokens[mkt.pair.asset_b].transfer(sender, &mkt.id, amount_b) {
                panic!("Corrupt Game");
            }

            trace!("Funds Returned");
            tokens[mkt.pair.asset_b].end_transaction();
            tokens[mkt.pair.asset_a].end_transaction();
            return Err(TradeError::InvalidTrade(
                "Reserves Not Preserved".to_string(),
            ));
        }

        let pair = game
            .swap
            .markets
            .get_mut(&mkt.pair)
            .expect("Market must exist");
        pair.reserve_a -= amount_a;
        pair.reserve_b -= amount_b;
        Ok(())
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
            std::mem::swap(&mut amount_a, &mut amount_b);
            id.normalize();
        }
        let mkt = ConstantFunctionMarketMakerPair::ensure(game, id);
        let tokens: &mut TokenRegistry = &mut game.tokens;

        let (buying, selling, sell_amt) = match (amount_a, amount_b) {
            (0, 0) => {
                return Err(TradeError::InvalidTrade(
                    "Both token amounts cannot be zero".into(),
                ))
            }
            (0, b) => (id.asset_a, id.asset_b, b),
            (a, 0) => (id.asset_b, id.asset_a, a),
            _ => {
                return Err(TradeError::InvalidTrade(
                    "One token amount must be zero".into(),
                ))
            }
        };

        // disabled this check: shouldn't matter?
        // if !(amount_a <= mkt.amt_a(tokens) && amount_b <= mkt.amt_b(tokens)) {
        //     return Err(TradeError::InsufficientTokens(
        //         "Market has insufficient tokens".into(),
        //     ));
        // }

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
            let buy_amt = mkt_qty_buying - k.roundup_div(mkt_qty_selling + sell_amt);
            let sell_amt = (k.roundup_div(mkt_qty_buying - buy_amt)) - mkt_qty_selling;
            if let Some(min) = buy_min {
                if buy_amt < min {
                    return Err(TradeError::MarketSlipped);
                }
            }
            if sell_amt > tokens[selling].balance_check(sender) {
                return Err(TradeError::InsufficientTokens(
                    "User has insufficient tokens".into(),
                ));
            }
            if !simulate {
                tokens[selling].transaction();
                if !tokens[selling].transfer(sender, &mkt.id, sell_amt) {
                    panic!("Logic Error: Invariant (Enough Balance) Already Checked");
                }
                tokens[selling].end_transaction();
                if let Err(e) = Self::swap_helper(selling, buying, game, 0, buy_amt, sender) {
                    let tokens = &mut game.tokens;
                    trace!(sell_amt, "Returning Funds Outer");
                    tokens[selling].transaction();
                    let _ = tokens[selling].transfer(&mkt.id, sender, sell_amt);
                    tokens[selling].end_transaction();
                    trace!("Funds Returned Outer");
                    return Err(e);
                }
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
        let mkt = ConstantFunctionMarketMakerPair::ensure(game, id);
        let tokens: &mut TokenRegistry = &mut game.tokens;
        // get the CFMM pair
        let mkt_qty_a = mkt.amt_a(tokens);
        let mkt_qty_b = mkt.amt_b(tokens);

        (mkt_qty_a, mkt_qty_b)
    }

    fn swap_helper(
        selling: TokenPointer,
        buying: TokenPointer,
        game: &mut GameBoard,
        sell_amt: u128,
        buy_amt: u128,
        sender: &EntityID,
    ) -> Result<(), TradeError> {
        tracing::trace!(sell_amt, buy_amt, ?selling, ?buying, "Token Swap");
        let mut trade_pair = TradingPairID {
            asset_a: selling,
            asset_b: buying,
        };
        if trade_pair.is_normal() {
            Self::raw_swap(
                game,
                trade_pair,
                sell_amt,
                buy_amt,
                &CallContext { sender: *sender },
            )
        } else {
            trade_pair.normalize();
            Self::raw_swap(
                game,
                trade_pair,
                buy_amt,
                sell_amt,
                &CallContext { sender: *sender },
            )
        }
    }
}

/// A struct for passing token qty information to the UX for price calculation
#[derive(Serialize, Clone, Debug, JsonSchema)]
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

trait DivExt:
    Div<Self, Output = Self> + Sized + Sub<Self, Output = Self> + Add<Self, Output = Self> + Copy
{
    fn unit() -> Self;
    fn roundup_div(self, rhs: Self) -> Self {
        (self + rhs - Self::unit()) / rhs
    }
}
impl DivExt for u128 {
    fn unit() -> Self {
        1
    }
}
