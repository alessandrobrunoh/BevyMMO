//! Network protocol types and application mode.
//!
//! Contains only data types, message definitions, and the [`ProtocolPlugin`]
//! registration. No transport: sockets live in `bevymmo_server` / `bevymmo_client`.

pub mod mode;
pub mod protocol;
