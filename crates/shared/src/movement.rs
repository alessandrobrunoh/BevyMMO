//! Shared pure helpers for point-and-click movement.
//!
//! Contains the canonical movement-speed computation, the cast-blocking
//! policy, and the shared move-towards-target stepping used by both the
//! authoritative server system and the client-side prediction system.

use bevy::prelude::{Mut, Resource, Vec3};

use crate::entity::components::EntityState;
use crate::network::protocol::{Inputs, LookDirection, Position};
use crate::spells::{CastKind, CastProgress};
use crate::stats::events::ModifierOp;
use crate::stats::events::StatField;
use crate::stats::modifiers::ActiveStatModifiers;
use crate::stats::modifiers::StatModifierInstance;

/// Distance (in world units) under which a move command is considered satisfied.
pub const ARRIVAL_DISTANCE: f32 = 0.05;

/// Local pending click target shared between click selection, input buffering,
/// and the predicted/authoritative move systems.
///
/// Pure data: lives in `shared` so both `bevymmo_server` and `bevymmo_client`
/// can use it without creating a cross-crate dependency.
#[derive(Resource, Default)]
pub struct MoveTarget(pub Option<Vec3>);

/// Calculates movement speed after active stat modifiers.
///
/// This is shared by gameplay and the stats UI so the value displayed to the
/// player matches the speed used by gameplay.
pub fn effective_movement_speed(base_speed: f32, modifiers: Option<&ActiveStatModifiers>) -> f32 {
    let Some(active) = modifiers else {
        return base_speed;
    };
    effective_value(StatField::Speed, base_speed, &active.modifiers)
}

fn effective_value(field: StatField, base: f32, modifiers: &[StatModifierInstance]) -> f32 {
    let mut result = base;
    let mut override_value: Option<f32> = None;

    for modifier in modifiers {
        for effect in &modifier.effects {
            if let crate::stats::modifiers::ModifierEffectInstance::Stat {
                field: effect_field,
                operation,
                value,
            } = effect
            {
                if *effect_field != field {
                    continue;
                }
                match operation {
                    ModifierOp::Add => result += value,
                    ModifierOp::Multiply => result *= value,
                    ModifierOp::Override => override_value = Some(*value),
                }
            }
        }
    }

    override_value.unwrap_or(result)
}

/// Returns true when a cast state must freeze point-and-click movement.
pub fn should_block_movement_for_cast(cast: Option<&CastProgress>) -> bool {
    let Some(cast) = cast else {
        return false;
    };
    match cast.kind {
        CastKind::CastTime => true,
        CastKind::Channeling => {
            cast.channel_movement == crate::spells::ChannelMovementPolicy::InterruptOnMove
        }
        CastKind::Instant => false,
    }
}

/// Steps a single entity towards its current move target.
///
/// Shared by the authoritative server system (`bevymmo_server::player_movement`)
/// and the client prediction system (`bevymmo_presentation::player_movement`)
/// so both sides advance movement with identical math.
///
/// Returns early if the entity is dead; clears state to `Idle` when the input
/// is not a `MoveTo` or the entity has reached the target.
pub fn move_towards_target(
    mut position: Mut<Position>,
    mut look_direction: Mut<LookDirection>,
    input: &Inputs,
    speed: f32,
    mut state: Mut<EntityState>,
) {
    if state.is_dead() {
        return;
    }

    let Inputs::MoveTo(target) = input else {
        *state = EntityState::Idle;
        return;
    };

    let offset = *target - position.0;
    let distance = offset.length();
    if distance > 0.001 {
        look_direction.0 = (offset / distance).normalize_or_zero();
    }
    if distance <= ARRIVAL_DISTANCE {
        position.0 = *target;
        *state = EntityState::Idle;
        return;
    }

    position.0 += offset / distance * speed.min(distance);
    *state = EntityState::Moving;
}
