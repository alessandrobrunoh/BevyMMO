//! Concrete `BaseAbility` implementations — un file per gesto, mirror di
//! `crate::spells_impl`/`crate::items_impl`.

pub mod arcane_gale;
pub mod arcane_orb;
pub mod arcane_seal;
pub mod binding_seal;
pub mod meteor_strike;


use crate::abilities::BaseAbilityRegistry;

/// Registra ogni gesto base disponibile. Chiamato una volta a Startup, sia
/// client sia server (stesso pattern di `register_default_items`).
/// Builds the registry containing every entry this build ships.
///
/// Returns the registry rather than filling a Bevy `Resource`: the
/// SpacetimeDB module has no `Startup` schedule and no ECS to put one in.
/// `bevymmo_shared` wraps this in a system for the client.
pub fn default_base_abilities() -> BaseAbilityRegistry {
    #[allow(unused_mut)]
    let mut registry = BaseAbilityRegistry::default();
    arcane_orb::ArcaneOrb::register(&mut registry);
    arcane_seal::ArcaneSeal::register(&mut registry);
    binding_seal::BindingSeal::register(&mut registry);
    arcane_gale::ArcaneGale::register(&mut registry);
    meteor_strike::MeteorStrike::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_abilities_populates_the_registry() {
        // No Bevy `App` any more: the registry is a plain value, so the test
        // that used to spin up a world and a schedule is now a function call.
        assert_eq!(default_base_abilities().len(), 5);
    }
}
