//! Events emitted by gameplay systems to apply Crowd Control to a target.

use bevy::ecs::entity::Entity;
use bevy::prelude::Message;

use bevymmo_shared::crowd_control::CrowdControlKind;

/// Request to apply a CC effect to `target`.
///
/// Emitted by the AoE system on detonation, and in the future by direct-hit
/// spells, traps, or boss mechanics. Consumed server-side by
/// [`crate::crowd_control::systems::apply_cc_events`].
///
/// # Example
/// ```rust,ignore
/// cc_events.write(ApplyCrowdControlEvent {
///     target,
///     source: Some(caster),
///     kind: CrowdControlKind::Stun,
///     duration_seconds: 2.0,
/// });
/// ```
#[derive(Debug, Clone, PartialEq, Message)]
pub struct ApplyCrowdControlEvent {
    pub target: Entity,
    pub source: Option<Entity>,
    pub kind: CrowdControlKind,
    pub duration_seconds: f32,
}
