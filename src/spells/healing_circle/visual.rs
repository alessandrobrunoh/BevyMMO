//! Visual client-side per la spell Healing Circle.
//!
//! Il dispatcher in `plugins::spells::effects` chiama `spawn` quando arriva un
//! `SpellVisualEffect` con l'id di Healing Circle.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::SpellVisualEffect;
use crate::plugins::spells::SpellVisual;
use crate::spells::healing_circle::HealingCircleSpell;

const DURATION_SECONDS: f32 = 3.0;

#[derive(Component)]
pub struct HealingCircleVisual {
    elapsed_seconds: f32,
    duration_seconds: f32,
}

/// Spawn della rappresentazione visiva di Healing Circle.
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let mesh = meshes.add(Cylinder::new(HealingCircleSpell::AREA_RADIUS, 0.1));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.9, 0.2, 0.4),
        emissive: LinearRgba::rgb(0.0, 0.5, 0.1),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(effect.start),
        SpellVisual,
        HealingCircleVisual {
            elapsed_seconds: 0.0,
            duration_seconds: DURATION_SECONDS,
        },
    ));
}

/// Anima le entità `HealingCircleVisual` (rotazione leggera) e le despawna a
/// fine durata.
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: Query<(Entity, &mut Transform, &mut HealingCircleVisual)>,
) {
    let delta = time.delta().as_secs_f32();
    for (entity, mut transform, mut visual) in visuals.iter_mut() {
        visual.elapsed_seconds += delta;

        let t = visual.elapsed_seconds / visual.duration_seconds;
        if t >= 1.0 {
            commands.entity(entity).despawn();
        } else {
            transform.rotate_y(delta * 0.5);
        }
    }
}
