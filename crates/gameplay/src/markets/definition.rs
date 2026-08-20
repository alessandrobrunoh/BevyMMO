//! Market identity, allowlist, and the pure fill planner.

use std::borrow::Cow;

use crate::economy::{quote_fee, FeeQuote, Gold, GoldError};
use crate::items::registry::ItemId;
use crate::registry::Registry;

/// How close a character must stand to an in-game market NPC, squared.
/// Same 6-unit radius the greeter uses (`6² = 36`).
pub const MARKET_PROXIMITY_SQUARED: f32 = 36.0;

/// Open sell + buy orders a character may hold in one market at once.
pub const MAX_ORDERS_PER_CHARACTER_PER_MARKET: usize = 10;

pub const MARKET_1_ID: &str = "market_1";
pub const MARKET_2_ID: &str = "market_2";
pub const MARKET_1_FEE_BPS: u16 = 200;
pub const MARKET_2_FEE_BPS: u16 = 300;

/// Stable market id (`market_1`, `market_2`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MarketId(Cow<'static, str>);

impl MarketId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Static description of one isolated market.
#[derive(Debug, Clone)]
pub struct MarketDefinition {
    pub id: MarketId,
    pub display_name: Cow<'static, str>,
    pub fee_bps: u16,
    pub allowed_item_ids: Vec<ItemId>,
}

impl MarketDefinition {
    pub fn allows(&self, item_id: &ItemId) -> bool {
        self.allowed_item_ids.iter().any(|id| id == item_id)
    }
}

/// Lookup table of markets, keyed by [`MarketId`].
#[derive(Debug, Clone, Default)]
pub struct MarketRegistry {
    inner: Registry<String, MarketDefinition>,
}

impl MarketRegistry {
    pub fn register(&mut self, market: MarketDefinition) {
        self.inner.insert(market.id.as_str().to_string(), market);
    }

    pub fn get(&self, id: &str) -> Option<&MarketDefinition> {
        self.inner.get(&id.to_string())
    }

    pub fn contains(&self, id: &str) -> bool {
        self.inner.contains(&id.to_string())
    }

    pub fn iter(&self) -> impl Iterator<Item = &MarketDefinition> {
        self.inner.iter().map(|(_, market)| market)
    }
}

/// Why a market action is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketError {
    UnknownMarket,
    ItemNotAllowed,
    WrongMarket,
    SelfTrade,
    InsufficientGold,
    InventoryFull,
    OrderCap,
    ZeroPrice,
    FeeExceedsPrice,
    NoMatchingBid,
}

impl std::fmt::Display for MarketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownMarket => write!(f, "unknown market"),
            Self::ItemNotAllowed => write!(f, "that item cannot be traded in this market"),
            Self::WrongMarket => write!(f, "that order belongs to a different market"),
            Self::SelfTrade => write!(f, "you cannot fill your own order"),
            Self::InsufficientGold => write!(f, "not enough gold"),
            Self::InventoryFull => write!(f, "inventory is full"),
            Self::OrderCap => write!(f, "too many open orders in this market"),
            Self::ZeroPrice => write!(f, "price must be greater than 0"),
            Self::FeeExceedsPrice => write!(f, "fee exceeds price"),
            Self::NoMatchingBid => write!(f, "no matching buy order"),
        }
    }
}

impl From<GoldError> for MarketError {
    fn from(err: GoldError) -> Self {
        match err {
            GoldError::Insufficient => Self::InsufficientGold,
            GoldError::ZeroPrice => Self::ZeroPrice,
            GoldError::FeeExceedsPrice => Self::FeeExceedsPrice,
            GoldError::Overflow => Self::InsufficientGold,
        }
    }
}

/// Planned gold movement for one fill. The reducer applies this in one
/// transaction: buyer pays `quote.buyer_pays`, seller receives
/// `quote.seller_receives`, the rest is burned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillPlan {
    pub quote: FeeQuote,
}

/// Validates a Buy of an existing sell order. Isolation (`order_market` vs
/// `npc_market`) and self-trade are checked before gold.
pub fn plan_fill(
    buyer_character_eq_seller: bool,
    order_market: &str,
    acting_market: &str,
    price: u64,
    market_bps: u16,
    seller_account_bps: u16,
    buyer_gold: Gold,
    buyer_has_free_slot: bool,
) -> Result<FillPlan, MarketError> {
    if order_market != acting_market {
        return Err(MarketError::WrongMarket);
    }
    if buyer_character_eq_seller {
        return Err(MarketError::SelfTrade);
    }
    if !buyer_has_free_slot {
        return Err(MarketError::InventoryFull);
    }
    let quote = quote_fee(price, market_bps, seller_account_bps)?;
    buyer_gold.debit(quote.buyer_pays.amount())?;
    Ok(FillPlan { quote })
}

/// Escrows `price` gold for a buy order. Fees are quoted later, on fill.
pub fn plan_place_buy_order(price: u64, buyer_gold: Gold) -> Result<Gold, MarketError> {
    if price == 0 {
        return Err(MarketError::ZeroPrice);
    }
    buyer_gold.debit(price)?;
    Ok(Gold::from_u64(price))
}

/// One bid as the fill planner sees it. `is_own` is the caller's bid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuyBid {
    pub id: u64,
    pub market_id: String,
    pub item_id: String,
    pub price_gold: u64,
    pub is_own: bool,
}

/// Highest bid of `market_id`+`item_id` with `price_gold >= min_price`.
/// Tie-break: lower `id`. Own bids never fill; if they are the only matches
/// that is [`MarketError::SelfTrade`], not a listing.
pub fn select_best_buy_order<'a>(
    bids: impl IntoIterator<Item = &'a BuyBid>,
    market_id: &str,
    item_id: &str,
    min_price: u64,
) -> Result<&'a BuyBid, MarketError> {
    let mut saw_own = false;
    let mut best: Option<&BuyBid> = None;
    for bid in bids {
        if bid.market_id != market_id || bid.item_id != item_id || bid.price_gold < min_price {
            continue;
        }
        if bid.is_own {
            saw_own = true;
            continue;
        }
        let take = match best {
            None => true,
            Some(current) => {
                bid.price_gold > current.price_gold
                    || (bid.price_gold == current.price_gold && bid.id < current.id)
            }
        };
        if take {
            best = Some(bid);
        }
    }
    match best {
        Some(bid) => Ok(bid),
        None if saw_own => Err(MarketError::SelfTrade),
        None => Err(MarketError::NoMatchingBid),
    }
}

/// Instant sell into an existing bid. Buyer already escrowed the bid;
/// seller is paid `quote.seller_receives` of **the bid price**.
pub fn plan_fill_buy_order(
    seller_character_eq_buyer: bool,
    order_market: &str,
    acting_market: &str,
    bid_price: u64,
    market_bps: u16,
    seller_account_bps: u16,
    buyer_has_free_slot: bool,
) -> Result<FillPlan, MarketError> {
    if order_market != acting_market {
        return Err(MarketError::WrongMarket);
    }
    if seller_character_eq_buyer {
        return Err(MarketError::SelfTrade);
    }
    if !buyer_has_free_slot {
        return Err(MarketError::InventoryFull);
    }
    let quote = quote_fee(bid_price, market_bps, seller_account_bps)?;
    Ok(FillPlan { quote })
}

pub fn assert_item_allowed(market: &MarketDefinition, item_id: &ItemId) -> Result<(), MarketError> {
    if market.allows(item_id) {
        Ok(())
    } else {
        Err(MarketError::ItemNotAllowed)
    }
}

pub fn assert_order_cap(open_orders: usize) -> Result<(), MarketError> {
    if open_orders >= MAX_ORDERS_PER_CHARACTER_PER_MARKET {
        Err(MarketError::OrderCap)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn market_one() -> MarketDefinition {
        MarketDefinition {
            id: MarketId::new(MARKET_1_ID),
            display_name: "Market 1".into(),
            fee_bps: MARKET_1_FEE_BPS,
            allowed_item_ids: vec![ItemId::new("sword"), ItemId::new("bow")],
        }
    }

    #[test]
    fn market_one_rejects_armor() {
        let market = market_one();
        assert!(market.allows(&ItemId::new("sword")));
        assert_eq!(
            assert_item_allowed(&market, &ItemId::new("simple_helm")),
            Err(MarketError::ItemNotAllowed)
        );
    }

    #[test]
    fn fill_rejects_cross_market_ids() {
        let err = plan_fill(
            false,
            MARKET_1_ID,
            MARKET_2_ID,
            1_000,
            MARKET_1_FEE_BPS,
            100,
            Gold::from_u64(10_000),
            true,
        )
        .unwrap_err();
        assert_eq!(err, MarketError::WrongMarket);
    }

    #[test]
    fn fill_rejects_self_trade_before_touching_gold() {
        let err = plan_fill(
            true,
            MARKET_1_ID,
            MARKET_1_ID,
            1_000,
            MARKET_1_FEE_BPS,
            100,
            Gold::ZERO,
            true,
        )
        .unwrap_err();
        assert_eq!(err, MarketError::SelfTrade);
    }

    #[test]
    fn fill_rejects_full_inventory() {
        let err = plan_fill(
            false,
            MARKET_1_ID,
            MARKET_1_ID,
            1_000,
            MARKET_1_FEE_BPS,
            100,
            Gold::from_u64(10_000),
            false,
        )
        .unwrap_err();
        assert_eq!(err, MarketError::InventoryFull);
    }

    #[test]
    fn fill_quotes_market_plus_account_fee() {
        // 2% + 1% of 10_000 = 300 burned, seller 9_700, buyer pays 10_000.
        let plan = plan_fill(
            false,
            MARKET_1_ID,
            MARKET_1_ID,
            10_000,
            MARKET_1_FEE_BPS,
            100,
            Gold::from_u64(10_000),
            true,
        )
        .unwrap();
        assert_eq!(plan.quote.fee_gold, 300);
        assert_eq!(plan.quote.seller_receives.amount(), 9_700);
        assert_eq!(
            plan.quote.buyer_pays.amount(),
            plan.quote.seller_receives.amount() + plan.quote.fee_gold
        );
    }

    #[test]
    fn fill_rejects_insufficient_gold() {
        let err = plan_fill(
            false,
            MARKET_1_ID,
            MARKET_1_ID,
            10_000,
            MARKET_1_FEE_BPS,
            100,
            Gold::from_u64(9_999),
            true,
        )
        .unwrap_err();
        assert_eq!(err, MarketError::InsufficientGold);
    }

    #[test]
    fn order_cap_rejects_at_the_limit() {
        assert!(assert_order_cap(9).is_ok());
        assert_eq!(
            assert_order_cap(MAX_ORDERS_PER_CHARACTER_PER_MARKET),
            Err(MarketError::OrderCap)
        );
    }

    #[test]
    fn isolated_registries_do_not_share_allowlists() {
        let mut registry = MarketRegistry::default();
        registry.register(market_one());
        registry.register(MarketDefinition {
            id: MarketId::new(MARKET_2_ID),
            display_name: "Market 2".into(),
            fee_bps: MARKET_2_FEE_BPS,
            allowed_item_ids: vec![ItemId::new("simple_helm")],
        });
        let one = registry.get(MARKET_1_ID).unwrap();
        let two = registry.get(MARKET_2_ID).unwrap();
        assert!(one.allows(&ItemId::new("sword")));
        assert!(!two.allows(&ItemId::new("sword")));
        assert!(two.allows(&ItemId::new("simple_helm")));
        assert!(!one.allows(&ItemId::new("simple_helm")));
    }

    fn bid(id: u64, market: &str, item: &str, price: u64, is_own: bool) -> BuyBid {
        BuyBid {
            id,
            market_id: market.to_string(),
            item_id: item.to_string(),
            price_gold: price,
            is_own,
        }
    }

    #[test]
    fn place_buy_order_debits_the_bid() {
        let remaining = plan_place_buy_order(1_000, Gold::from_u64(1_500)).unwrap();
        assert_eq!(remaining.amount(), 1_000);
    }

    #[test]
    fn place_buy_order_rejects_zero_and_insufficient_gold() {
        assert_eq!(
            plan_place_buy_order(0, Gold::from_u64(10_000)),
            Err(MarketError::ZeroPrice)
        );
        assert_eq!(
            plan_place_buy_order(1_000, Gold::from_u64(999)),
            Err(MarketError::InsufficientGold)
        );
    }

    #[test]
    fn two_bids_pick_the_highest_price() {
        let bids = [
            bid(1, MARKET_1_ID, "sword", 100, false),
            bid(2, MARKET_1_ID, "sword", 250, false),
        ];
        let best = select_best_buy_order(&bids, MARKET_1_ID, "sword", 1).unwrap();
        assert_eq!(best.id, 2);
        assert_eq!(best.price_gold, 250);
    }

    #[test]
    fn equal_prices_tie_break_to_the_lower_id() {
        let bids = [
            bid(8, MARKET_1_ID, "sword", 100, false),
            bid(3, MARKET_1_ID, "sword", 100, false),
        ];
        let best = select_best_buy_order(&bids, MARKET_1_ID, "sword", 100).unwrap();
        assert_eq!(best.id, 3);
    }

    #[test]
    fn self_fill_is_rejected_when_only_own_bids_match() {
        let bids = [
            bid(1, MARKET_1_ID, "sword", 500, true),
            bid(2, MARKET_1_ID, "bow", 900, false),
        ];
        let err = select_best_buy_order(&bids, MARKET_1_ID, "sword", 1).unwrap_err();
        assert_eq!(err, MarketError::SelfTrade);
    }

    #[test]
    fn cross_market_bids_are_ignored() {
        let bids = [bid(1, MARKET_2_ID, "sword", 9_000, false)];
        let err = select_best_buy_order(&bids, MARKET_1_ID, "sword", 1).unwrap_err();
        assert_eq!(err, MarketError::NoMatchingBid);
    }

    #[test]
    fn no_matching_bid_is_an_error_not_a_listing() {
        let bids = [bid(1, MARKET_1_ID, "sword", 50, false)];
        let err = select_best_buy_order(&bids, MARKET_1_ID, "sword", 100).unwrap_err();
        assert_eq!(err, MarketError::NoMatchingBid);
    }

    #[test]
    fn fill_buy_order_quotes_the_bid_price() {
        let plan = plan_fill_buy_order(
            false,
            MARKET_1_ID,
            MARKET_1_ID,
            10_000,
            MARKET_1_FEE_BPS,
            100,
            true,
        )
        .unwrap();
        assert_eq!(plan.quote.buyer_pays.amount(), 10_000);
        assert_eq!(plan.quote.fee_gold, 300);
        assert_eq!(plan.quote.seller_receives.amount(), 9_700);
    }

    #[test]
    fn fill_buy_order_zero_account_fee_leaves_only_the_market_cut() {
        let plan = plan_fill_buy_order(
            false,
            MARKET_1_ID,
            MARKET_1_ID,
            10_000,
            MARKET_1_FEE_BPS,
            0,
            true,
        )
        .unwrap();
        assert_eq!(plan.quote.total_bps, 200);
        assert_eq!(plan.quote.fee_gold, 200);
        assert_eq!(plan.quote.seller_receives.amount(), 9_800);
    }

    #[test]
    fn fill_buy_order_rejects_self_cross_market_and_full_inventory() {
        assert_eq!(
            plan_fill_buy_order(true, MARKET_1_ID, MARKET_1_ID, 100, 200, 0, true).unwrap_err(),
            MarketError::SelfTrade
        );
        assert_eq!(
            plan_fill_buy_order(false, MARKET_1_ID, MARKET_2_ID, 100, 200, 0, true).unwrap_err(),
            MarketError::WrongMarket
        );
        assert_eq!(
            plan_fill_buy_order(false, MARKET_1_ID, MARKET_1_ID, 100, 200, 0, false).unwrap_err(),
            MarketError::InventoryFull
        );
    }
}
