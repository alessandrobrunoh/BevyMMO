//! Boss spawn definition (shared data).
//!
//! Re-exported from `bevymmo_shared`: `impl Boss` (arena constants, ability
//! list) and `impl EntityDefinition for Boss` moved to the shared crate so
//! the orphan rule is respected.

pub use bevymmo_shared::entity::boss::spawn::*;
