//! Placeable kinds — re-exported from [`bevymmo_domain::placeables`].
//!
//! The definitions moved to the domain crate so the SpacetimeDB module can seed
//! the world from the same registry the client renders from. Nothing about the
//! API changed; this module exists so that callers keep writing
//! `bevymmo_shared::placeables::...`.
pub use bevymmo_domain::placeables::*;
