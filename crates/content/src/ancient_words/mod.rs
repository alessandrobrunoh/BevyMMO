//! Ancient-word content and its registry.
//!
//! No ancient word has been implemented yet.

use crate::abilities::AncientWordRegistry;

/// Builds the registry containing every ancient word shipped by this game build.
pub fn default_ancient_words() -> AncientWordRegistry {
    AncientWordRegistry::default()
}
