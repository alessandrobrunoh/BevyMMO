//! The two isolated markets shipped with this build.

use bevymmo_gameplay::items::registry::ItemId;
use bevymmo_gameplay::markets::{
    MarketDefinition, MarketId, MarketRegistry, MARKET_1_FEE_BPS, MARKET_1_ID, MARKET_2_FEE_BPS,
    MARKET_2_ID,
};

/// Market 1: weapons. Market 2: a subset of armor. Isolated order books.
pub fn default_markets() -> MarketRegistry {
    let mut registry = MarketRegistry::default();
    registry.register(MarketDefinition {
        id: MarketId::new(MARKET_1_ID),
        display_name: "Market 1".into(),
        fee_bps: MARKET_1_FEE_BPS,
        allowed_item_ids: vec![
            ItemId::new("bow"),
            ItemId::new("sword"),
            ItemId::new("hammer"),
            ItemId::new("mage_staff"),
        ],
    });
    registry.register(MarketDefinition {
        id: MarketId::new(MARKET_2_ID),
        display_name: "Market 2".into(),
        fee_bps: MARKET_2_FEE_BPS,
        allowed_item_ids: vec![
            ItemId::new("simple_helm"),
            ItemId::new("simple_cuirass"),
            ItemId::new("simple_buckler"),
        ],
    });
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markets_are_isolated() {
        let registry = default_markets();
        let one = registry.get(MARKET_1_ID).unwrap();
        let two = registry.get(MARKET_2_ID).unwrap();
        assert!(one.allows(&ItemId::new("sword")));
        assert!(!two.allows(&ItemId::new("sword")));
        assert!(two.allows(&ItemId::new("simple_helm")));
        assert!(!one.allows(&ItemId::new("simple_helm")));
    }
}
