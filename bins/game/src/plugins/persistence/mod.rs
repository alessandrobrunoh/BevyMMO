//! PostgreSQL persistence layer facade.
//!
//! The source of truth now lives in `bevymmo_server::persistence`. This root
//! module is kept as a compatibility layer so existing imports keep working
//! during the crate-split migration.

pub use bevymmo_server::persistence::*;
