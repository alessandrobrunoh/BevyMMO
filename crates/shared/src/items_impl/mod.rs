//! Concrete built-in item implementations.
//!
//! Each submodule is a self-contained `Item` trait implementation with no
//! transport/rendering dependencies, so the registry in the binary (or any
//! other crate) can compose them freely. Mirrors `crate::spells_impl`.

pub mod iron_sword;

use std::sync::Arc;

use crate::items::registry::ItemRegistry;

/// Registers every item definition available to the current game build.
///
/// Called once at startup by both the server (authoritative source) and the
/// client (UI rendering). Keeping the list in `shared` guarantees both sides
/// agree on what items exist.
pub fn register_default_items(mut registry: bevy::prelude::ResMut<ItemRegistry>) {
    registry.register(Arc::new(iron_sword::IronSword::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::registry::ItemId;

    #[test]
    fn register_default_items_adds_iron_sword() {
        let mut app = bevy::prelude::App::new();
        app.init_resource::<ItemRegistry>();
        app.add_systems(bevy::prelude::Update, register_default_items);
        app.update();

        let registry = app.world().resource::<ItemRegistry>();
        assert!(
            registry.contains(&ItemId::new(iron_sword::IronSword::ID)),
            "iron_sword must be registered after register_default_items"
        );
        assert!(!registry.is_empty());
    }
}
