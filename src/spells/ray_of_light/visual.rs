//! Client-side visual for the "Ray of Light" spell.
//!
//! The server resolves the start/end of the beam (always along the caster's
//! facing direction, up to max range). The client constructs an emissive yellow
//! ray connecting the two points, faded out rapidly. Damage was already
//! applied server-side: this entity is purely decorative.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::SpellVisualEffect;
use crate::plugins::spells::SpellVisual;

/// Beam flash duration. Short: the ray is a flash, not a persistent effect.
pub const LIFETIME_SECONDS: f32 = 0.18;

/// Vertical thickness of the beam. Very thin to give the impression of a blade of light.
const BEAM_HEIGHT: f32 = 0.15;
/// Width of the beam. Slightly less than the player (as requested by design).
const BEAM_WIDTH: f32 = 0.6;
/// Raises the beam off the floor to align it with the caster's center of mass.
const SPAWN_HEIGHT_OFFSET: f32 = 0.8;
/// Initial transparency of the beam. The animation fades it out to 0.
const INITIAL_ALPHA: f32 = 0.95;

#[derive(Component)]
pub struct RayOfLightVisual {
    elapsed_seconds: f32,
    initial_alpha: f32,
}

/// Constructs a local beam from the segment resolved by the server.
///
/// The visual has no authority over damage: it only replicates the segment communicated
/// by the server so that the client sees the ray stop exactly at max range.
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
    let start = effect.start + Vec3::Y * SPAWN_HEIGHT_OFFSET;
    let end = effect.end + Vec3::Y * SPAWN_HEIGHT_OFFSET;
    let displacement = end - start;
    let length = displacement.length();
    let unit_length = length.max(0.001);

    let mesh = meshes.add(Cuboid::new(BEAM_WIDTH, BEAM_HEIGHT, unit_length));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.95, 0.2, INITIAL_ALPHA),
        emissive: LinearRgba::rgb(1.0, 0.9, 0.15),
        unlit: true,
        ..default()
    });

    // Orientation: the Cuboid is long along Z, so we align +Z with displacement.
    let forward = displacement / unit_length;
    let orientation = Quat::from_rotation_arc(Vec3::Z, forward);
    // Center the beam halfway between start and end.
    let midpoint = start + displacement * 0.5;

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(midpoint).with_rotation(orientation),
        SpellVisual,
        RayOfLightVisual {
            elapsed_seconds: 0.0,
            initial_alpha: INITIAL_ALPHA,
        },
    ));
}

/// Fades out the beam until despawning. Main-world only system.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, animate);
/// ```
#[allow(clippy::type_complexity)]
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut visuals: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        &mut RayOfLightVisual,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let delta = time.delta().as_secs_f32();
    for (entity, material_handle, mut visual) in visuals.iter_mut() {
        visual.elapsed_seconds += delta;
        let progress = (visual.elapsed_seconds / LIFETIME_SECONDS).clamp(0.0, 1.0);

        let Some(mut material) = materials.get_mut(material_handle) else {
            // Safety fallback: despawn if material was deallocated.
            if progress >= 1.0 {
                commands.entity(entity).despawn();
            }
            continue;
        };

        let alpha = visual.initial_alpha * (1.0 - progress);
        material.base_color.set_alpha(alpha.max(0.0));

        if progress >= 1.0 {
            commands.entity(entity).despawn();
        }
    }
}
