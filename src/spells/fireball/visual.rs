//! Visual client-side per la spell Fireball.
//!
//! Il dispatcher in `plugins::spells::effects` chiama `spawn` quando arriva un
//! `SpellVisualEffect` con l'id di Fireball. L'animazione è registrata nello
//! stesso punto.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::SpellVisualEffect;
use crate::plugins::spells::SpellVisual;

const DURATION_SECONDS: f32 = 0.35;
const SIZE: f32 = 0.28;

#[derive(Component)]
pub struct FireballVisual {
    start: Vec3,
    end: Vec3,
    elapsed_seconds: f32,
    duration_seconds: f32,
}

/// Spawn della rappresentazione visiva di Fireball.
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let mesh = meshes.add(Cuboid::new(SIZE, SIZE, SIZE));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.35, 0.05),
        emissive: LinearRgba::rgb(1.0, 0.25, 0.02),
        ..default()
    });

    let start = effect.start + Vec3::Y * 0.8;
    let end = effect.end + Vec3::Y * 0.8;

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(start),
        SpellVisual,
        FireballVisual {
            start,
            end,
            elapsed_seconds: 0.0,
            duration_seconds: DURATION_SECONDS,
        },
    ));
}

/// Anima le entità `FireballVisual` verso il target e le despawna a fine corsa.
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: Query<(Entity, &mut Transform, &mut FireballVisual)>,
) {
    let delta = time.delta().as_secs_f32();
    for (entity, mut transform, mut visual) in visuals.iter_mut() {
        visual.elapsed_seconds += delta;
        let t = (visual.elapsed_seconds / visual.duration_seconds).clamp(0.0, 1.0);
        transform.translation = visual.start.lerp(visual.end, t);
        transform.scale = Vec3::splat(1.0 + t * 0.6);

        if t >= 1.0 {
            commands.entity(entity).despawn();
        }
    }
}
