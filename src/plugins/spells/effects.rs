//! Client-only visual spell effects.
//!
//! Questo modulo è solo un **dispatcher**: legge i messaggi `SpellVisualEffect`
//! replicati dal server e invoca la funzione `spawn` del modulo `visual` della
//! spell corrispondente (es. `crate::spells::fireball::visual`).
//!
//! Le singole spell contengono sia il componente marker visivo che le relative
//! funzioni di spawn/animazione. Il cleanup centralizzato si basa sul marker
//! `SpellVisual` applicato a ogni entità visiva.

use bevy::prelude::*;

use crate::game_state::{GameScreen, Screen};
use crate::network::mode::has_client;
use crate::network::protocol::SpellVisualEffect;

use crate::spells::fireball::FireballSpell;
use crate::spells::healing_circle::HealingCircleSpell;
use crate::spells::meteorite::MeteoriteSpell;
use crate::spells::swift::SwiftSpell;

/// Marker applicato a tutte le entità visual delle spell. Permette il cleanup
/// centralizzato quando si esce dal gameplay.
#[derive(Component)]
pub struct SpellVisual;

pub fn client_effect_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            spawn_spell_visuals,
            crate::spells::fireball::visual::animate,
            crate::spells::healing_circle::visual::animate,
            crate::spells::meteorite::visual::animate,
            crate::spells::swift::visual::animate,
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
    mut commands: Commands,
    mut messages: MessageReader<SpellVisualEffect>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for effect in messages.read() {
        match effect.spell_id.as_str() {
            FireballSpell::ID => {
                crate::spells::fireball::visual::spawn(
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
            SwiftSpell::ID => {
                crate::spells::swift::visual::spawn(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    effect,
                );
            }
            other => {
                bevy::log::debug!("SpellVisualEffect senza visual registrato: {other}");
            }
        }
    }
}

/// Despawna tutte le entità visual quando non si è più in gameplay.
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
            spell_id: "fireball".to_string(),
            start: Vec3::ZERO,
            end: Vec3::Z,
        };
        assert_eq!(effect.start, Vec3::ZERO);
        assert_eq!(effect.end, Vec3::Z);
    }
}
