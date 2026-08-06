//! Network protocol types and registration (shared data).
//!
//! Re-exported from `bevymmo_shared`: `Position`, messages, channels and
//! `ProtocolPlugin` now live in the shared crate so both server and client
//! agree on the same protocol without duplicating types.

pub use bevymmo_shared::network::protocol::*;
