//! Engine-independent gameplay rules and frameworks.

pub mod abilities;
pub mod crowd_control;
pub mod effects;
pub mod entity;
pub mod items;
pub mod placeables;
pub mod registry;
pub mod spells;
pub mod stats;
pub mod movement;

pub use bevymmo_core::{EntityId, Rgba};
pub use bevymmo_core::{ids, math};
pub use bevymmo_world as world;
