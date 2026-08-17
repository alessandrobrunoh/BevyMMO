//! Network protocol types and application mode.
//!
//! Contains only data types and message definitions used to talk to the
//! SpacetimeDB module. No transport lives here: the SpacetimeDB SDK owns the
//! connection, in `bevymmo_client::stdb`.

pub mod mode;
pub mod protocol;
