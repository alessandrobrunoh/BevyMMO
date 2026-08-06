//! Client-side visual for the Fireball spell.
//!
//! The server resolves the first hit point, then replicates a visual effect with
//! a start and end position. The client only animates that authoritative segment
//! so the projectile disappears exactly where the gameplay hit happened.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::SpellVisualEffect;
use crate::plugins::spells::SpellVisual;

/// Safety cap for orphaned visuals if a malformed effect has a zero-length path.
pub const LIFETIME_SECONDS: f32 = 2.0;

/// Flight speed in world units per second.
pub const SPEED: f32 = 32.0;

const SIZE: f32 = 0.28;
const SPAWN_HEIGHT_OFFSET: f32 = 0.8;

#[derive(Component)]
pub struct FireballVisual {
    velocity: Vec3,
    target: Vec3,
    elapsed_seconds: f32,
}

/// Builds a short-lived local projectile from the server-resolved impact path.
///
/// The visual has no authority over damage. It only mirrors the segment chosen
/// by the server so the client sees the ball stop on the first entity hit.
///
/// # Example
/// ```rust,ignore
/// spawn(&mut commands, &mut meshes, &mut materials, &effect);
/// ```
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

    let start = effect.start + Vec3::Y * SPAWN_HEIGHT_OFFSET;
    let target = effect.end + Vec3::Y * SPAWN_HEIGHT_OFFSET;
    let direction = (target - start).try_normalize().unwrap_or(Vec3::Z);

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(start),
        SpellVisual,
        FireballVisual {
            velocity: direction * SPEED,
            target,
            elapsed_seconds: 0.0,
        },
    ));
}

/// Advances each fireball until it reaches the authoritative end point.
///
/// The fallback lifetime avoids leaked entities if a bad network payload creates
/// a degenerate path. This system runs on the Bevy main world only.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, animate);
/// ```
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: Query<(Entity, &mut Transform, &mut FireballVisual)>,
) {
    let delta = time.delta().as_secs_f32();
    for (entity, mut transform, mut visual) in visuals.iter_mut() {
        visual.elapsed_seconds += delta;
        let remaining = visual.target - transform.translation;
        let remaining_distance = remaining.length();
        let step = visual.velocity.length() * delta;

        if step >= remaining_distance || visual.elapsed_seconds >= LIFETIME_SECONDS {
            transform.translation = visual.target;
            commands.entity(entity).despawn();
            continue;
        }

        transform.translation += visual.velocity * delta;
    }
}
