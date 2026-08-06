//! Stats events (shared data).
//!
//! Re-exported from `bevymmo_shared`: damage/heal/modifier events moved to
//! the shared crate so spells and gameplay systems can emit them from any
//! crate.

pub use bevymmo_shared::stats::events::*;
