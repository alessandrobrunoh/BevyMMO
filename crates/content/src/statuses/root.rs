//! Root: hard control that prevents movement but allows casting.

use bevymmo_props_macro::status;

use crate::effects::StatusRegistry;

#[status(
    id = "root",
    name = "Root",
    icon = "status_root",
    category = Debuff,
    duration = 2.5,
    cleanseable = true,
    stacking = Refresh,
    refresh = RefreshAll,
    control = Root
)]
pub struct Root;

pub fn register(registry: &mut StatusRegistry) {
    Root::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{ControlSpec, DispelPolicy, Status, StatusCategory};

    #[test]
    fn root_is_a_cleanseable_movement_control_debuff() {
        let definition = Root::definition();

        assert_eq!(definition.category, StatusCategory::Debuff);
        assert!(definition.cleanseable);
        assert!(!definition.purgeable);
        assert_eq!(definition.dispel, DispelPolicy::RemoveWholeStatus);
        assert_eq!(definition.control, Some(ControlSpec::Root));
        assert_eq!(definition.duration_seconds, 2.5);
        assert!(definition.stat_modifiers.is_empty());
    }
}
