//! Swift: a non-cleanseable movement-speed Buff.

use bevymmo_props_macro::status;

use crate::effects::StatusRegistry;

#[status(
    id = "swift",
    name = "Swift",
    icon = "status_swift",
    category = Buff,
    duration = 8.0,
    cleanseable = false,
    purgeable = true,
    stacking = Refresh,
    refresh = RefreshAll,
    modifier(
        stat = Speed,
        operation = Add,
        value = 0.25
    )
)]
pub struct Swift;

pub fn register(registry: &mut StatusRegistry) {
    Swift::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{Status, StatusCategory};
    use crate::stats::events::{ModifierOp, StatField};

    #[test]
    fn swift_is_a_non_cleanseable_speed_buff() {
        let definition = Swift::definition();

        assert_eq!(definition.category, StatusCategory::Buff);
        assert!(!definition.cleanseable);
        assert!(definition.purgeable);
        assert_eq!(definition.stat_modifiers.len(), 1);
        assert_eq!(definition.stat_modifiers[0].field, StatField::Speed);
        assert_eq!(definition.stat_modifiers[0].operation, ModifierOp::Add);
        assert_eq!(definition.stat_modifiers[0].value, 0.25);
    }
}
