//! Placeable catalog: the single source of truth for "what can be placed
//! in the world".
//!
//! Each kind has a [`PlaceableDefinition`] (shared data contract) and
//! optionally a server binding (gameplay behavior) and a client binding
//! (rendering). This mirrors the spell framework (`spells/` +
//! `content/spells/` + [`crate::spells::SpellRegistry`]): one trait, a typed
//! registry, and concrete definitions in [`crate::content::placeables`].
//!
//! ## Categories as subtraits
//!
//! Unlike the spell catalog (one flat trait), placeable kinds are split
//! into category **subtraits** ([`PropPlaceable`], [`EnemyPlaceable`],
//! [`BossPlaceable`], ...). Implementing a subtrait IS the categorization:
//! the compiler guarantees every registered enemy has `enemy_config()`, so
//! the server dispatches via a typed HashMap lookup instead of a runtime
//! `match` on an enum. Adding a new `mob_orc` is two impl blocks — no
//! central edit.
//!
//! ## Composition with the entity system
//!
//! The catalog does not subclass [`crate::entity::definition::EntityDefinition`].
//! That trait stays for runtime gameplay entities (`Player`, `Enemy`, `Boss`).
//! Instead, the server binding layer (in `bevymmo_server::placeables`)
//! reads the catalog DTOs and calls the existing
//! [`crate::entity::spawn::spawn_entity`] helper, layering catalog-provided
//! configuration on top. See `plans/placeable-catalog.md` (D5, D9) for the
//! full rationale.

pub mod category;
pub mod config;
pub mod definition;
pub mod registry;

pub use category::PlaceableCategory;
pub use config::{
    BossConfig, EnemyConfig, InteractionKind, ResourceConfig, TriggerConfig, TriggerEvent,
    TriggerShape,
};
pub use definition::{
    AssetHint, BossPlaceable, EnemyPlaceable, InteractablePlaceable, NpcPlaceable,
    PlaceableDefaults, PlaceableDefinition, PlayerSpawnPlaceable, PropPlaceable,
    ResourceNodePlaceable, TriggerPlaceable,
};
pub use registry::{KindId, PlaceableRegistry};

// Re-export the procedural macro for ergonomic prop definition
pub use bevymmo_props_macro::props;
