//! Application role and run conditions (shared data).
//!
//! Re-exported from `bevymmo_shared`: the source of truth moved to the shared
//! crate so every role (server/client/editor) agrees on the same `AppMode`.

pub use bevymmo_shared::network::mode::*;
