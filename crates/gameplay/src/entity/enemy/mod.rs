//! `enemy` entity data.

pub mod aggro;
pub mod components;
pub mod kit;
pub mod pick;
pub mod threat;

pub use aggro::{
    acquire_center, acquires_by_proximity, horizontal_distance, in_acquire_radius, is_leashed,
    select_target, AcquirePolicy, AggroOrigin, AggroProfile, ThreatCandidate, ThreatPolicy,
};
pub use kit::{AbilityTargeting, AbilityUse};
pub use pick::pick_ability;
pub use threat::threat_from_damage;
