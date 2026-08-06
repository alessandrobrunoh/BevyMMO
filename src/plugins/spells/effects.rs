//! Client-only visual spell effects.
//!
//! This module is purely a **dispatcher**: it reads `SpellVisualEffect` messages
//! replicated from the server and calls the `spawn` function in the corresponding
//! spell's `visual` module (e.g. `crate::spells::ray_of_light::visual`).
//!
//! Individual spells contain both the visual marker component and related
//! spawn/animation functions. Centralized cleanup is based on the
//! `SpellVisual` marker placed on each visual entity.

use bevy::prelude::*;

use crate::game_state::{GameScreen, Screen};
use crate::network::mode::has_client;
use crate::network::protocol::SpellVisualEffect;

use crate::plugins::spells::{CastKind, SpellHudCooldownStarted, SpellId, SpellRegistry};
use crate::spells::healing_circle::HealingCircleSpell;
use crate::spells::meteorite::MeteoriteSpell;
use crate::spells::ray_of_light::RayOfLightSpell;
use crate::spells::stun_field::StunFieldSpell;

/// Marker placed on all spell visual entities. Enables centralized cleanup
/// when leaving gameplay.
#[derive(Component)]
pub struct SpellVisual;

struct RecentSpellVisual {
    spell_id: String,
    start: Vec3,
    end: Vec3,
    seen_at_seconds: f32,
}

pub fn client_effect_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            spawn_spell_visuals,
            crate::spells::ray_of_light::visual::animate,
            crate::spells::healing_circle::visual::animate,
            crate::spells::meteorite::visual::animate,
            crate::spells::stun_field::visual::animate,
        )
            .chain()
            .run_if(has_client)
            .run_if(in_gameplay),
    );
    app.add_systems(
        Update,
        cleanup_spell_visuals
            .run_if(has_client)
            .run_if(not_in_gameplay),
    );
}

fn in_gameplay(screen: Res<GameScreen>) -> bool {
    matches!(screen.0, Screen::InGame | Screen::Paused)
}

fn not_in_gameplay(screen: Res<GameScreen>) -> bool {
    !in_gameplay(screen)
}

fn spawn_spell_visuals(
    time: Res<Time>,
    mut commands: Commands,
    mut messages: MessageReader<SpellVisualEffect>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    registry: Res<SpellRegistry>,
    mut hud_cooldowns: MessageWriter<SpellHudCooldownStarted>,
    mut recent_visuals: Local<Vec<RecentSpellVisual>>,
) {
    let now = time.elapsed_secs();
    recent_visuals.retain(|recent| now - recent.seen_at_seconds <= 0.25);

    for effect in messages.read() {
        if !should_spawn_visual(&mut recent_visuals, effect, now) {
            continue;
        }

        start_authoritative_cooldown(&registry, &mut hud_cooldowns, effect.spell_id.as_str());

        match effect.spell_id.as_str() {
            RayOfLightSpell::ID => {
                crate::spells::ray_of_light::visual::spawn(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    effect,
                );
            }
            HealingCircleSpell::ID => {
                crate::spells::healing_circle::visual::spawn(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    effect,
                );
            }
            MeteoriteSpell::ID => {
                crate::spells::meteorite::visual::spawn(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    effect,
                );
            }
            StunFieldSpell::ID => {
                crate::spells::stun_field::visual::spawn(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    effect,
                );
            }
            other => {
                bevy::log::debug!("SpellVisualEffect without registered visual: {other}");
            }
        }
    }
}

/// Starts HUD cooldowns only after the server accepted and emitted a visual effect.
///
/// Cast-time abilities can be interrupted after the key press. Waiting for this
/// authoritative signal prevents false cooldowns when the server cancels the cast.
///
/// # Example
/// ```rust,ignore
/// start_authoritative_cooldown(&registry, &mut hud_cooldowns, "meteorite");
/// ```
fn start_authoritative_cooldown(
    registry: &SpellRegistry,
    hud_cooldowns: &mut MessageWriter<SpellHudCooldownStarted>,
    spell_id: &str,
) {
    let Some(spell_def) = registry.get(&SpellId::new(spell_id.to_owned())) else {
        return;
    };
    let config = spell_def.config();
    if spell_def.cast_kind() == CastKind::Instant || config.cooldown_seconds <= 0.0 {
        return;
    }

    hud_cooldowns.write(SpellHudCooldownStarted {
        spell_id: SpellId::new(spell_id.to_owned()),
        cooldown_seconds: config.cooldown_seconds,
    });
}

/// Filters duplicate visuals that can appear in host-client mode.
///
/// The server writes local ECS messages for immediate feedback and also sends
/// the same message through Lightyear. Without a short dedupe window, host-client
/// can spawn two identical Meteorite circles and reset HUD cooldowns twice.
///
/// # Example
/// ```rust,ignore
/// if should_spawn_visual(&mut recent_visuals, &effect, now) {
///     // spawn render entity
/// }
/// ```
fn should_spawn_visual(
    recent_visuals: &mut Vec<RecentSpellVisual>,
    effect: &SpellVisualEffect,
    now: f32,
) -> bool {
    let is_duplicate = recent_visuals.iter().any(|recent| {
        recent.spell_id == effect.spell_id
            && recent.start.distance_squared(effect.start) <= 0.0001
            && recent.end.distance_squared(effect.end) <= 0.0001
    });
    if is_duplicate {
        return false;
    }

    recent_visuals.push(RecentSpellVisual {
        spell_id: effect.spell_id.clone(),
        start: effect.start,
        end: effect.end,
        seen_at_seconds: now,
    });
    true
}

/// Despawns all visual entities when not in gameplay.
fn cleanup_spell_visuals(mut commands: Commands, visuals: Query<Entity, With<SpellVisual>>) {
    for entity in visuals.iter() {
        commands.entity(entity).despawn();
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_visual_effect_stores_start_and_end_points() {
        let effect = SpellVisualEffect {
            spell_id: "ray_of_light".to_string(),
            start: Vec3::ZERO,
            end: Vec3::Z,
        };
        assert_eq!(effect.start, Vec3::ZERO);
        assert_eq!(effect.end, Vec3::Z);
    }
}
