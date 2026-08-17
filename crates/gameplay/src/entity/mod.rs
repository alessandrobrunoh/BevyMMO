//! Entity data shared by client and server.
//!
//! Only the data. Spawning — bundles, meshes, replication targets — is Bevy's
//! business and stayed in `bevymmo_shared::entity`.

pub mod boss;
pub mod components;
pub mod dummy;
pub mod enemy;
pub mod player;

pub use components::{EntityKind, EntityState, GameEntity, PlayerName, SpawnPoint};
