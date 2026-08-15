//! Shared entity types: markers, state, names, and spawn contracts.
//!
//! These types are used by every crate: the server spawns them, the network
//! replicates them, and the presentation layer reads them to build visuals.

pub mod boss;
pub use bevymmo_domain::entity::components;
pub mod definition;
pub mod dummy;
pub mod enemy;
pub use bevymmo_domain::entity::events;
pub mod player;
pub mod spawn;

/// Marks the entity this client controls.
///
/// Replaces lightyear's `Controlled`, which the UI used to answer "is this me?".
/// The transport now supplies the answer — the SpacetimeDB bridge inserts this
/// on the entity whose row matches the connection's `Identity` — but the
/// question is a presentation one, so the marker lives here rather than in the
/// networking crate that happens to set it.
#[derive(bevy::prelude::Component, Debug, Clone, Copy)]
pub struct LocalPlayer;
