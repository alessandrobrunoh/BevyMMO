//! Ancient-word content and its registry.
//!
pub mod amplia;

use crate::abilities::AncientWordRegistry;

/// Builds the registry containing every ancient word shipped by this game build.
pub fn default_ancient_words() -> AncientWordRegistry {
    let mut registry = AncientWordRegistry::default();
    amplia::Amplia::register(&mut registry);
    registry
}
