//! Burn: a cleanseable periodic damage status.

use bevymmo_props_macro::status;

use crate::effects::StatusRegistry;

#[status(
    id = "burn",
    name = "Burn",
    icon = "status_burn",
    category = Debuff,
    duration = 5.0,
    cleanseable = true,
    stacking = AddStacks,
    stack_scope = PerSource,
    max_stacks = 5,
    refresh = RefreshAll,
    periodic(
        interval = 1.0,
        amount = 10.0
    )
)]
pub struct Burn;

pub fn register(registry: &mut StatusRegistry) {
    Burn::register(registry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{PeriodicEffect, StackPolicy, StackScope, Status, StatusCategory};

    #[test]
    fn burn_is_periodic_damage_with_per_source_stacks() {
        let definition = Burn::definition();

        assert_eq!(definition.category, StatusCategory::Debuff);
        assert_eq!(definition.stacking, StackPolicy::AddStacks);
        assert_eq!(definition.stack_scope, StackScope::PerSource);
        assert_eq!(definition.periodic.unwrap().interval_seconds, 1.0);
        assert_eq!(
            definition.periodic.unwrap().effect,
            PeriodicEffect::Damage { amount: 10.0 }
        );
    }
}
