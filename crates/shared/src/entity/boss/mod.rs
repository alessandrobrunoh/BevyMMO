//! Boss encounter components (dragon boss).
//!
//! `Boss`, `BossPhase` and `BossArena` are replicated so clients can render
//! the arena ring, boss bar and phase banner. The remaining types are
//! server-only AI state and never cross the network.

pub mod components;
pub mod spawn;
