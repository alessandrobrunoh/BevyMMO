//! The two isolated markets shipped with this build.

use bevymmo_gameplay::markets::{
    MarketDefinition, MarketId, MarketRegistry, MARKET_1_FEE_BPS, MARKET_1_ID, MARKET_2_FEE_BPS,
    MARKET_2_ID,
};

/// Two halls with isolated order books and different fees. Neither restricts
/// what it accepts: any item flagged `tradable` can be listed in either.
pub fn default_markets() -> MarketRegistry {
    let mut registry = MarketRegistry::default();
    registry.register(MarketDefinition {
        id: MarketId::new(MARKET_1_ID),
        display_name: "Market 1".into(),
        fee_bps: MARKET_1_FEE_BPS,
    });
    registry.register(MarketDefinition {
        id: MarketId::new(MARKET_2_ID),
        display_name: "Market 2".into(),
        fee_bps: MARKET_2_FEE_BPS,
    });
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_gameplay::items::registry::ItemId;
    use bevymmo_gameplay::markets::assert_item_marketable;

    #[test]
    fn halls_are_isolated_by_fee_not_by_catalogue() {
        let registry = default_markets();
        let one = registry.get(MARKET_1_ID).unwrap();
        let two = registry.get(MARKET_2_ID).unwrap();
        assert_eq!(one.fee_bps, MARKET_1_FEE_BPS);
        assert_eq!(two.fee_bps, MARKET_2_FEE_BPS);
        assert_ne!(one.fee_bps, two.fee_bps);
    }

    /// A gathered material is the case that used to fail: `tradable = true`,
    /// but on nobody's allowlist, so it could not be sold anywhere.
    #[test]
    fn every_tradable_item_can_be_listed() {
        let items = crate::item_definitions::default_items();
        let wood = items.get(&ItemId::new("wood")).expect("wood is registered");
        assert!(wood.tradable());
        assert!(assert_item_marketable(wood.tradable()).is_ok());
    }
}
