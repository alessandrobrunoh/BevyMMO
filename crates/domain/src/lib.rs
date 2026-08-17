//! Compatibility facade during the crate-split migration.
//!
//! New code should depend directly on `bevymmo_core`, `bevymmo_world`,
//! `bevymmo_gameplay`, or `bevymmo_content`.

pub mod content;
pub use bevymmo_gameplay::{
    abilities, crowd_control, effects, entity, items, movement, placeables, spells, stats,
};
pub use bevymmo_gameplay::{EntityId, Rgba};
pub use bevymmo_world as world;
