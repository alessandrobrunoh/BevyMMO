//! Stun: hard control that prevents movement and casting.

use bevymmo_props_macro::status;

use crate::effects::StatusRegistry;

#[status(
    id = "stun",
    name = "Stun",
    icon = "status_stun",
    category = Debuff,
    duration = 2.0,
    cleanseable = true,
    stacking = Refresh,
    refresh = RefreshAll,
    control = Stun
)]
pub struct Stun;

pub fn register(registry: &mut StatusRegistry) {
    Stun::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{ControlSpec, DispelPolicy, Status, StatusCategory};

    #[test]
    fn stun_is_a_cleanseable_debuff_with_hard_control() {
        let definition = Stun::definition();

        assert_eq!(definition.category, StatusCategory::Debuff);
        assert!(definition.cleanseable);
        assert_eq!(definition.dispel, DispelPolicy::RemoveWholeStatus);
        assert_eq!(definition.control, Some(ControlSpec::Stun));
        assert_eq!(definition.duration_seconds, 2.0);
    }
}
