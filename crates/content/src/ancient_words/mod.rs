//! Ancient-word content and its registry.
//!

pub mod echo;
pub mod twin;
pub mod return_dir;
pub mod hunger;
pub mod anchor;
pub mod reversal;

use crate::abilities::AncientWordRegistry;

/// Builds the registry containing every ancient word shipped by this game build.
pub fn default_ancient_words() -> AncientWordRegistry {
    let mut registry = AncientWordRegistry::default();

    echo::Echo::register(&mut registry);
    twin::Twin::register(&mut registry);
    return_dir::ReturnWord::register(&mut registry);
    hunger::Hunger::register(&mut registry);
    anchor::Anchor::register(&mut registry);
    reversal::Reversal::register(&mut registry);
    registry
}
