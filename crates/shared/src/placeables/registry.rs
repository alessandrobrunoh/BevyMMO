//! Central registry of all placeable kinds.
//!
//! Mirrors [`crate::spells::SpellRegistry`]: one resource populated at
//! startup, looked up by [`KindId`] during map validation and spawn.
//!
//! Unlike the spell registry, this one stores definitions in **typed
//! submaps** per category subtrait (`PropPlaceable`, `EnemyPlaceable`, ...).
//! Dispatch is a typed HashMap lookup — the compiler guarantees every
//! registered enemy has `enemy_config()`, so there is no central `match`
//! to edit when adding a new kind.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::category::PlaceableCategory;
use super::definition::{
    BossPlaceable, EnemyPlaceable, InteractablePlaceable, NpcPlaceable, PlayerSpawnPlaceable,
    PropPlaceable, ResourceNodePlaceable, TriggerPlaceable,
};

// -------------------------------------------------------------------------
// KindId
// -------------------------------------------------------------------------

/// Stable unique identifier for a placeable kind.
///
/// Newtype around a string, mirroring [`crate::spells::SpellId`]. Stored in
/// the map manifest; the loader validates it against the registry.
///
/// Serializes transparently as a string, so existing `.ron` files with
/// `kind: "tree_oak"` keep loading unchanged.
///
/// # Example
///
/// ```rust,ignore
/// use bevymmo_shared::placeables::KindId;
/// let id: KindId = "tree_oak".into();
/// assert_eq!(id.as_str(), "tree_oak");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KindId(pub(crate) Cow<'static, str>);

impl KindId {
    /// Create a new `KindId` from a static or owned string.
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }

    /// The underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for KindId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for KindId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// -------------------------------------------------------------------------
// Registry
// -------------------------------------------------------------------------

/// Central registry of every placeable kind, grouped into typed submaps
/// by category subtrait.
///
/// Populated at startup by `register_default_placeables()` (see the
/// `placeables_impl` module). Looked up by:
/// - the editor palette (to list kinds grouped by category),
/// - the map loader (to validate `kind_id`s),
/// - the server spawn machinery (to dispatch creature placements),
/// - the client binding (to resolve asset hints).
///
/// Each category has its own typed `register_*` / lookup path. Adding a
/// new goblin variant is `register_enemy(Arc::new(OrcDefinition))` — no
/// enum edit, no match arm.
#[derive(Resource, Default)]
pub struct PlaceableRegistry {
    /// Static visual props.
    pub props: HashMap<KindId, Arc<dyn PropPlaceable>>,
    /// Hostile / neutral AI creatures.
    pub enemies: HashMap<KindId, Arc<dyn EnemyPlaceable>>,
    /// Boss entities.
    pub bosses: HashMap<KindId, Arc<dyn BossPlaceable>>,
    /// Friendly interactable NPCs.
    pub npcs: HashMap<KindId, Arc<dyn NpcPlaceable>>,
    /// Player spawn markers.
    pub player_spawns: HashMap<KindId, Arc<dyn PlayerSpawnPlaceable>>,
    /// Invisible trigger zones.
    pub triggers: HashMap<KindId, Arc<dyn TriggerPlaceable>>,
    /// Harvestable resource nodes.
    pub resources: HashMap<KindId, Arc<dyn ResourceNodePlaceable>>,
    /// One-shot interactables (doors, chests, levers).
    pub interactables: HashMap<KindId, Arc<dyn InteractablePlaceable>>,
}

impl PlaceableRegistry {
    // --- typed register methods ------------------------------------------

    /// Registers a static prop kind.
    pub fn register_prop(&mut self, def: Arc<dyn PropPlaceable>) {
        self.props.insert(def.id(), def);
    }

    /// Registers an enemy archetype.
    pub fn register_enemy(&mut self, def: Arc<dyn EnemyPlaceable>) {
        self.enemies.insert(def.id(), def);
    }

    /// Registers a boss archetype.
    pub fn register_boss(&mut self, def: Arc<dyn BossPlaceable>) {
        self.bosses.insert(def.id(), def);
    }

    /// Registers an interactive NPC.
    pub fn register_npc(&mut self, def: Arc<dyn NpcPlaceable>) {
        self.npcs.insert(def.id(), def);
    }

    /// Registers a player spawn marker.
    pub fn register_player_spawn(&mut self, def: Arc<dyn PlayerSpawnPlaceable>) {
        self.player_spawns.insert(def.id(), def);
    }

    /// Registers a trigger zone kind.
    pub fn register_trigger(&mut self, def: Arc<dyn TriggerPlaceable>) {
        self.triggers.insert(def.id(), def);
    }

    /// Registers a resource node kind.
    pub fn register_resource(&mut self, def: Arc<dyn ResourceNodePlaceable>) {
        self.resources.insert(def.id(), def);
    }

    /// Registers a one-shot interactable.
    pub fn register_interactable(&mut self, def: Arc<dyn InteractablePlaceable>) {
        self.interactables.insert(def.id(), def);
    }

    // --- lookups ----------------------------------------------------------

    /// Returns `true` if any category submap contains `id`.
    ///
    /// Used by the map loader to validate `kind_id`s without caring which
    /// category they belong to.
    pub fn contains(&self, id: &KindId) -> bool {
        self.props.contains_key(id)
            || self.enemies.contains_key(id)
            || self.bosses.contains_key(id)
            || self.npcs.contains_key(id)
            || self.player_spawns.contains_key(id)
            || self.triggers.contains_key(id)
            || self.resources.contains_key(id)
            || self.interactables.contains_key(id)
    }

    /// Total number of registered kinds across all categories.
    pub fn len(&self) -> usize {
        self.props.len()
            + self.enemies.len()
            + self.bosses.len()
            + self.npcs.len()
            + self.player_spawns.len()
            + self.triggers.len()
            + self.resources.len()
            + self.interactables.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// UI hint: which palette group does `id` belong to?
    ///
    /// **Dispatch never uses this** — it uses the typed submaps directly.
    /// This method exists only so the editor palette can group entries
    /// without re-implementing the "which submap holds this id" check.
    pub fn category_of(&self, id: &KindId) -> Option<PlaceableCategory> {
        if self.props.contains_key(id) {
            Some(PlaceableCategory::Prop)
        } else if self.enemies.contains_key(id)
            || self.bosses.contains_key(id)
            || self.npcs.contains_key(id)
            || self.player_spawns.contains_key(id)
        {
            Some(PlaceableCategory::Creature)
        } else if self.triggers.contains_key(id) {
            Some(PlaceableCategory::Trigger)
        } else if self.resources.contains_key(id) {
            Some(PlaceableCategory::ResourceNode)
        } else if self.interactables.contains_key(id) {
            Some(PlaceableCategory::Interactable)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placeables::{
        AssetHint, PlaceableCategory, PlaceableDefaults, PlaceableDefinition, PropPlaceable,
    };

    struct DummyProp {
        id: &'static str,
        name: &'static str,
    }

    impl PlaceableDefinition for DummyProp {
        fn id(&self) -> KindId {
            KindId::new(self.id)
        }
        fn display_name(&self) -> &'static str {
            self.name
        }
        fn asset_hint(&self) -> AssetHint {
            AssetHint::Placeholder
        }
        fn defaults(&self) -> PlaceableDefaults {
            PlaceableDefaults::default()
        }
    }

    impl PropPlaceable for DummyProp {}

    #[test]
    fn register_and_lookup_prop() {
        let mut registry = PlaceableRegistry::default();
        registry.register_prop(Arc::new(DummyProp {
            id: "test_prop",
            name: "Test Prop",
        }));

        let id = KindId::new("test_prop");
        assert!(registry.contains(&id));
        assert_eq!(registry.category_of(&id), Some(PlaceableCategory::Prop));
        let def = registry.props.get(&id).expect("prop registered");
        assert_eq!(def.display_name(), "Test Prop");
    }

    #[test]
    fn unknown_kind_is_not_found() {
        let registry = PlaceableRegistry::default();
        let id = KindId::new("does_not_exist");
        assert!(!registry.contains(&id));
        assert_eq!(registry.category_of(&id), None);
    }

    #[test]
    fn kind_id_serializes_transparently_as_string() {
        let id = KindId::new("tree_oak");
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "\"tree_oak\"");
        let back: KindId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, id);
    }

    #[test]
    fn kind_id_from_static_str() {
        let id: KindId = "rock_01".into();
        assert_eq!(id.as_str(), "rock_01");
    }

    #[test]
    fn empty_registry_reports_empty() {
        let registry = PlaceableRegistry::default();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn len_counts_all_categories() {
        let mut registry = PlaceableRegistry::default();
        registry.register_prop(Arc::new(DummyProp { id: "a", name: "A" }));
        registry.register_prop(Arc::new(DummyProp { id: "b", name: "B" }));
        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
    }
}
