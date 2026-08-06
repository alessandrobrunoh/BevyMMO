//! Server-authoritative spell runtime.
//!
//! Owns the cast pipeline, persistent AoE lifecycle, homing projectile updates,
//! and other authoritative spell execution systems.

pub mod aoe;
pub mod projectile;
pub mod systems;
