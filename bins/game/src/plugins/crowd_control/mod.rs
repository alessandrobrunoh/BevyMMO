//! Server-authoritative Crowd Control (CC) framework.
//!
//! Shared replicated state (`CrowdControlState`, `CrowdControlKind`) lives in
//! `bevymmo_shared`; the server-owned event and systems live in
//! `bevymmo_server`. This root module is now a compatibility facade so the
//! existing import paths keep working during the crate-split migration.

pub mod components;
pub mod events;
pub mod systems;

pub use bevymmo_server::crowd_control::{ApplyCrowdControlEvent, CrowdControlPlugin};
pub use bevymmo_shared::crowd_control::{ActiveCrowdControl, CrowdControlKind, CrowdControlState};
