//! Client visual for `cataclysm`.
//!
//! Arena-wide red flash disc spawned each channel tick.
//! A giant cylinder fades from alpha 0.5 to 0 over 0.4s, creating
//! a pulsing red flash effect across the arena.

use bevy::color::Color;
use bevy::prelude::*;

use crate::network::protocol::SpellVisualEffect;
use crate::plugins::spells::SpellVisual;
use crate::spells::dragon_enemy::cataclysm::CataclysmSpell;
use crate::spells::dragon_enemy::FIRE_ORANGE;

const FLASH_SECONDS: f32 = 0.4;

#[derive(Component)]
pub struct CataclysmFlashVisual {
    elapsed_seconds: f32,
}

/// Spawns an arena-wide red flash disc at the caster position.
///
/// Each tick of the channeling spell spawns a new flash that fades
/// quickly, creating a pulsing arena-wide warning effect.
///
/// # Example
/// ```rust,ignore
/// visual::spawn(&mut commands, &mut meshes, &mut materials, &effect);
/// ```
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    effect: &SpellVisualEffect,
) {
    let center = effect.start + Vec3::Y * 0.1;

    let mesh = meshes.add(Cylinder::new(CataclysmSpell::AREA_RADIUS, 0.05));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.45, 0.05, 0.5),
        emissive: FIRE_ORANGE,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(center).with_scale(Vec3::splat(1.0)),
        SpellVisual,
        CataclysmFlashVisual {
            elapsed_seconds: 0.0,
        },
    ));
}

/// Animates the arena-wide red flash fade.
///
/// The flash fades from scale 1.0 to 0 over 0.4s, simulating alpha fade.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, visual::animate);
/// ```
pub fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut flashes: Query<(Entity, &mut Transform, &mut CataclysmFlashVisual)>,
) {
    let delta = time.delta_secs();

    for (entity, mut transform, mut visual) in flashes.iter_mut() {
        visual.elapsed_seconds += delta;
        let t = visual.elapsed_seconds;

        if t >= FLASH_SECONDS {
            commands.entity(entity).despawn();
            continue;
        }

        // Fade out via scale reduction
        let fade = 1.0 - (t / FLASH_SECONDS);
        let scale = fade.max(0.0);
        transform.scale = Vec3::new(scale, 1.0, scale);
    }
}
