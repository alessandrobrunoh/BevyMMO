//! Client-side feedback for Swift.
//!
//! The movement buff is server-authoritative. This module renders one lightweight
//! aura per observed Swift channel instead of spawning a new mesh every tick.

use bevy::prelude::*;

use crate::network::protocol::NetworkEntityId;
use crate::plugins::spells::cast_bar::ObservedCasts;
use crate::plugins::spells::SpellVisual;
use crate::spells::swift::SwiftSpell;

#[derive(Component)]
pub struct SwiftAura {
    caster_network_id: u64,
}

/// Maintains one aura while a caster is channeling Swift.
///
/// The cast progress stream is already authoritative and deduplicated by caster,
/// so using it avoids per-tick visual message spam while still giving immediate
/// feedback when `F` starts working.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, visual::sync_channel_aura);
/// ```
pub fn sync_channel_aura(
    mut commands: Commands,
    observed_casts: Res<ObservedCasts>,
    casters: Query<(Entity, &NetworkEntityId)>,
    auras: Query<(Entity, &SwiftAura)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (aura_entity, aura) in auras.iter() {
        if is_swift_active(&observed_casts, aura.caster_network_id) {
            continue;
        }
        commands.entity(aura_entity).despawn();
    }

    for (caster_entity, network_id) in casters.iter() {
        if !is_swift_active(&observed_casts, network_id.0) {
            continue;
        }
        if auras
            .iter()
            .any(|(_, aura)| aura.caster_network_id == network_id.0)
        {
            continue;
        }
        let aura_entity = spawn_aura(&mut commands, &mut meshes, &mut materials, network_id.0);
        commands
            .entity(aura_entity)
            .set_parent_in_place(caster_entity);
    }
}

/// Checks whether a replicated caster is currently channeling Swift.
///
/// # Example
/// ```rust,ignore
/// if is_swift_active(&observed_casts, caster_network_id) {
///     // keep the aura visible
/// }
/// ```
fn is_swift_active(observed_casts: &ObservedCasts, caster_network_id: u64) -> bool {
    observed_casts
        .0
        .get(&caster_network_id)
        .is_some_and(|cast| cast.spell_id == SwiftSpell::ID)
}

/// Animates the active Swift aura without allocating new render assets.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, visual::animate);
/// ```
pub fn animate(time: Res<Time>, mut auras: Query<&mut Transform, With<SwiftAura>>) {
    let pulse = 1.0 + (time.elapsed_secs() * 8.0).sin() * 0.08;
    for mut transform in auras.iter_mut() {
        transform.scale = Vec3::splat(pulse);
    }
}

/// Builds the render entity used for a Swift channel aura.
///
/// # Example
/// ```rust,ignore
/// let aura = spawn_aura(&mut commands, &mut meshes, &mut materials, network_id);
/// ```
fn spawn_aura(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    caster_network_id: u64,
) -> Entity {
    let mesh = meshes.add(Torus::new(0.75, 0.9));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.35, 0.75, 1.0, 0.55),
        emissive: LinearRgba::rgb(0.05, 0.35, 0.9),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(Vec3::Y * 0.08)
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::splat(1.0)),
            SpellVisual,
            SwiftAura { caster_network_id },
        ))
        .id()
}
