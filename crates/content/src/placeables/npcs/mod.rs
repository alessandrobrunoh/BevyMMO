//! Concrete NPC definitions.
//!
//! Populated by the catalog-extensions agent. Each NPC kind is a self-contained
//! definition registered at startup via [`register_all`].

pub mod greeter;
pub mod market;
pub mod merchant;

use crate::placeables::PlaceableRegistry;

/// Registers every NPC kind. Called by
/// `crate::content::placeables::register_all`.
pub fn register_all(registry: &mut PlaceableRegistry) {
    greeter::register(registry);
    merchant::register(registry);
    market::register(registry);
}
