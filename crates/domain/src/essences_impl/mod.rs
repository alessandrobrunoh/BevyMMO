//! Concrete `Essence` implementations — un file per Essenza, mirror di
//! `crate::spells_impl`. Aggiungere una nuova Essenza = nuovo file + una
//! riga qui, zero modifiche a codice centrale.

pub mod fuoco;
pub mod gelo;
pub mod terra;


use crate::abilities::EssenceRegistry;

/// Builds the registry containing every entry this build ships.
///
/// Returns the registry rather than filling a Bevy `Resource`: the
/// SpacetimeDB module has no `Startup` schedule and no ECS to put one in.
/// `bevymmo_shared` wraps this in a system for the client.
pub fn default_essences() -> EssenceRegistry {
    #[allow(unused_mut)]
    let mut registry = EssenceRegistry::default();
    fuoco::FuocoEssence::register(&mut registry);
    gelo::GeloEssence::register(&mut registry);
    terra::TerraEssence::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_essences_populates_the_registry() {
        // No Bevy `App` any more: the registry is a plain value, so the test
        // that used to spin up a world and a schedule is now a function call.
        assert_eq!(default_essences().len(), 3);
    }
}
