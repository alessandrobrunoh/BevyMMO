//! Concrete `Modifier` implementations — un file per Modificatore, mirror di
//! `crate::spells_impl`.

pub mod amplificare;
pub mod concentrare;
pub mod espandere;


use crate::abilities::ModifierRegistry;

/// Builds the registry containing every entry this build ships.
///
/// Returns the registry rather than filling a Bevy `Resource`: the
/// SpacetimeDB module has no `Startup` schedule and no ECS to put one in.
/// `bevymmo_shared` wraps this in a system for the client.
pub fn default_modifiers() -> ModifierRegistry {
    #[allow(unused_mut)]
    let mut registry = ModifierRegistry::default();
    espandere::EspandereModifier::register(&mut registry);
    amplificare::AmplificareModifier::register(&mut registry);
    concentrare::ConcentrareModifier::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_modifiers_populates_the_registry() {
        // No Bevy `App` any more: the registry is a plain value, so the test
        // that used to spin up a world and a schedule is now a function call.
        assert_eq!(default_modifiers().len(), 3);
    }
}
