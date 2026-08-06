//! Server-owned shared entity systems.
//!
//! Re-exported from `bevymmo_server`: server-authoritative shared entity
//! transitions (such as `mark_dead_entities`) now live in the server crate.

pub use bevymmo_server::gameplay::entity::systems::*;
