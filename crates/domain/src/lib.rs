//! Game rules and data types, independent of Bevy and of the runtime hosting them.
//!
//! See `Cargo.toml` for the constraints this crate operates under. The short
//! version: everything here must compile to `wasm32-unknown-unknown`.
//!
//! # Why there are no `SpacetimeType` derives here
//!
//! The obvious design is for these types to *be* the database row types, deriving
//! `SpacetimeType` so the module can store them directly. Two measured facts rule
//! that out:
//!
//! 1. **The derive rejects tuple structs.** `spacetimedb-bindings-macro 2.8.1`
//!    does `f.ident.unwrap()` on every field (`sats.rs:390`), which is `None` for
//!    unnamed fields, so the proc macro panics outright. This codebase is
//!    newtype-heavy on purpose — `SpellId`, `ItemId`, `EntityId`, `PlayerName`,
//!    `ItemInstanceId` — and flattening all of them into named-field structs
//!    would trade real type safety for the convenience of one derive.
//! 2. **The derive assumes it lives inside a module crate.** Its default path is
//!    `spacetimedb::spacetimedb_lib`, and it emits `__make_register_reftype!`,
//!    which registers the type in the module's global type registry. That
//!    registry is module machinery; it has no meaning in a crate the native
//!    client also links.
//!
//! So the boundary sits one layer out: the SpacetimeDB module declares its own
//! row structs with named fields and `SpacetimeType`, and converts to and from
//! these types with `From`/`Into`. That costs some boilerplate in the module and
//! buys this crate its independence — it stays a plain Rust library that the
//! client, the module, and the tests can all use without dragging a database
//! along.

pub mod abilities;
pub mod ancient_words_impl;
pub mod base_abilities_impl;
pub mod essences_impl;
pub mod items;
pub mod items_impl;
pub mod modifiers_impl;
pub mod placeables_impl;
pub mod spells_impl;
pub mod crowd_control;
pub mod entity;
pub mod ids;
pub mod math;
pub mod placeables;
pub mod spells;
pub mod stats;
pub mod world;

pub use ids::EntityId;
pub use math::Rgba;

/// Movement rules shared by the client's dead reckoning and the server's tick.
pub mod movement;
