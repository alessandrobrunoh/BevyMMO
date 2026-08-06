//! Server-side Lightyear transport and lifecycle.
//!
//! Re-exported from `bevymmo_server`: the authoritative server transport,
//! join/disconnect handling, persistence-backed player loading, and server
//! startup plugin moved to the server crate.

pub use bevymmo_server::network::server::*;
