//! Crowd control data types.
//!
//! The CC lifecycle systems (apply, tick, expiry) are server logic in
//! `bevymmo_server`; here live the replicated state and the event used to
//! request a CC application.

pub mod components;

pub use components::{ActiveCrowdControl, CrowdControlKind, CrowdControlState};
