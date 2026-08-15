//! Stats data — re-exported from [`bevymmo_domain::stats`].
//!
//! The types moved to the domain crate so the SpacetimeDB module can apply
//! damage, healing and modifiers with the same rules the client displays. The
//! `Component`/`Reflect` derives are still there: `bevymmo_shared` turns on the
//! domain crate's `bevy` feature.
pub use bevymmo_domain::stats::*;
