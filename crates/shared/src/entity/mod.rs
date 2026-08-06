//! Shared entity types: markers, state, names, and spawn contracts.
//!
//! These types are used by every crate: the server spawns them, the network
//! replicates them, and the presentation layer reads them to build visuals.

pub mod boss;
pub mod components;
pub mod definition;
pub mod dummy;
pub mod enemy;
pub mod events;
pub mod player;
pub mod spawn;
