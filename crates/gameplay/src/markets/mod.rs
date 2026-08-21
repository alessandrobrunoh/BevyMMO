//! Isolated player markets: order caps and fill planning.
//!
//! Each market has its own order book. A listing in Market 1 is invisible to
//! Market 2. Any item flagged `tradable` can be listed, bid on, or filled in
//! either of them.

pub mod definition;

pub use definition::{
    assert_item_marketable, assert_order_cap, plan_fill, plan_fill_buy_order, plan_place_buy_order,
    select_best_buy_order, BuyBid, FillPlan, MarketDefinition, MarketError, MarketId,
    MarketRegistry, MARKET_1_FEE_BPS, MARKET_1_ID, MARKET_2_FEE_BPS, MARKET_2_ID,
    MARKET_PROXIMITY_SQUARED, MAX_ORDERS_PER_CHARACTER_PER_MARKET,
};
