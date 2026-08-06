//! Client composite visual for the dragon boss.
//!
//! The generic renderer gives every game entity a small cuboid. For the boss we
//! build a larger composite (body, head, wings, eyes) as child entities under a
//! single `DragonRig` (itself a child of the boss), and animate the rig for the
//! per-phase behavior (idle bob, aerial elevation, berserk aura, death
//! collapse). Markers carry the boss `Entity` directly so we never walk the
//! parent hierarchy (the `Parent` component API is gone in Bevy 0.19).

use bevy::color::Color;
use bevy::prelude::*;

use super::components::{Boss, BossPhase};
use crate::game_state::{GameScreen, Screen};
use crate::network::mode::has_client;

/// Marker on the rig child that holds the dragon's animated body parts.
/// Stores the owning boss so the animator can look up the phase directly.
#[derive(Component)]
struct DragonRig {
    boss: Entity,
}

/// Marker on the pulsing berserk aura shell.
#[derive(Component)]
struct DragonAura {
    boss: Entity,
}

/// Per-boss lerp state so aerial/death transitions ease instead of snapping.
#[derive(Component, Default)]
struct DragonRigAnimation {
    death_seconds: f32,
}

pub fn client_dragon_systems(app: &mut App) {
    app.add_systems(
        Update,
        (spawn_dragon_parts, animate_dragon.after(spawn_dragon_parts))
            .run_if(has_client)
            .run_if(in_gameplay),
    )
    .add_systems(
        Update,
        cleanup_dragon_parts
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

/// Spawns the composite rig + body parts once per boss.
///
/// The rig is parented to the boss (so it inherits the boss's replicated world
/// transform) but carries `boss: Entity` on its marker so the animator resolves
/// the phase without walking the hierarchy.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, spawn_dragon_parts.run_if(has_client));
/// ```
fn spawn_dragon_parts(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    bosses: Query<Entity, With<Boss>>,
    existing_rigs: Query<&DragonRig>,
) {
    for boss in bosses.iter() {
        // Skip if this boss already has a rig (rig markers are not children in
        // the query, so check by scanning existing rig markers' boss field).
        if existing_rigs.iter().any(|rig| rig.boss == boss) {
            continue;
        }

        let body_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.05, 0.05),
            emissive: LinearRgba::rgb(0.20, 0.02, 0.02),
            ..default()
        });
        let wing_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.04, 0.04),
            emissive: LinearRgba::rgb(0.15, 0.02, 0.02),
            ..default()
        });
        let eye_material = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.85, 0.25),
            emissive: LinearRgba::rgb(1.0, 0.70, 0.20),
            ..default()
        });
        let aura_material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.95, 0.40, 0.05, 0.15),
            emissive: LinearRgba::rgb(0.95, 0.40, 0.05),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        let body_mesh = meshes.add(Cuboid::new(3.0, 2.0, 5.0));
        let head_mesh = meshes.add(Cuboid::new(1.5, 1.5, 1.5));
        let wing_mesh = meshes.add(Cuboid::new(4.0, 0.2, 2.5));
        let eye_mesh = meshes.add(Sphere::new(0.12));
        let aura_mesh = meshes.add(Sphere::new(4.0));

        commands.entity(boss).with_children(|parent| {
            parent
                .spawn((
                    Transform::default(),
                    Visibility::default(),
                    DragonRig { boss },
                    DragonRigAnimation::default(),
                ))
                .with_children(|rig| {
                    rig.spawn((
                        Mesh3d(body_mesh),
                        MeshMaterial3d(body_material.clone()),
                        Transform::from_xyz(0.0, 1.5, 0.0),
                    ));
                    rig.spawn((
                        Mesh3d(head_mesh),
                        MeshMaterial3d(body_material),
                        Transform::from_xyz(0.0, 2.5, 3.2),
                    ));
                    rig.spawn((
                        Mesh3d(wing_mesh.clone()),
                        MeshMaterial3d(wing_material.clone()),
                        Transform::from_xyz(-2.5, 2.5, 0.0),
                    ));
                    rig.spawn((
                        Mesh3d(wing_mesh),
                        MeshMaterial3d(wing_material),
                        Transform::from_xyz(2.5, 2.5, 0.0),
                    ));
                    rig.spawn((
                        Mesh3d(eye_mesh.clone()),
                        MeshMaterial3d(eye_material.clone()),
                        Transform::from_xyz(-0.4, 2.7, 3.9),
                    ));
                    rig.spawn((
                        Mesh3d(eye_mesh),
                        MeshMaterial3d(eye_material),
                        Transform::from_xyz(0.4, 2.7, 3.9),
                    ));
                    rig.spawn((
                        Mesh3d(aura_mesh),
                        MeshMaterial3d(aura_material),
                        Transform::from_xyz(0.0, 1.5, 0.0).with_scale(Vec3::ZERO),
                        DragonAura { boss },
                    ));
                });
        });
    }
}

/// Animates the rig: idle bob (dormant/ground), aerial lift, berserk aura
/// pulse, and death collapse. Resolves the boss phase via the rig's `boss` field.
///
/// The two queries are wrapped in a `ParamSet` because both mutably access
/// `Transform`; Bevy's B0001 check can't prove they target disjoint entities
/// (rig vs. its aura child), so we run them sequentially instead of in
/// parallel.
fn animate_dragon(
    time: Res<Time>,
    mut params: ParamSet<(
        Query<(&DragonRig, &mut Transform, &mut DragonRigAnimation)>,
        Query<(
            &DragonAura,
            &mut Transform,
            &MeshMaterial3d<StandardMaterial>,
        )>,
    )>,
    phases: Query<&BossPhase>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let t = time.elapsed_secs();
    let delta = time.delta_secs();

    {
        let mut rigs = params.p0();
        for (rig, mut transform, mut anim) in rigs.iter_mut() {
            let phase = phases.get(rig.boss).copied().unwrap_or(BossPhase::Dormant);

            let target_elevation = match phase {
                BossPhase::Dead => 0.0,
                BossPhase::Aerial | BossPhase::Berserk => 4.0,
                _ => 0.0,
            };
            let current = transform.translation.y;
            let eased = current + (target_elevation - current) * (delta * 2.0).min(1.0);

            let bob = if phase != BossPhase::Dead {
                (t * 1.5).sin() * 0.1
            } else {
                0.0
            };
            transform.translation.y = eased + bob;

            if phase == BossPhase::Dead {
                anim.death_seconds += delta;
                let p = (anim.death_seconds / 1.0).clamp(0.0, 1.0);
                let tip = (1.0 - (1.0 - p).powi(2)) * 1.4; // ~80° forward collapse
                transform.rotation = Quat::from_rotation_x(-tip);
            } else {
                anim.death_seconds = 0.0;
                // Gentle tilt to suggest facing the threat (visual only).
                transform.rotation = Quat::IDENTITY;
            }
        }
    }

    {
        let mut auras = params.p1();
        for (aura, mut aura_transform, aura_material) in auras.iter_mut() {
            let phase = phases.get(aura.boss).copied().unwrap_or(BossPhase::Dormant);
            if phase == BossPhase::Berserk {
                let pulse = 1.0 + (t * 6.0).sin() * 0.05;
                aura_transform.scale = Vec3::new(pulse, pulse, pulse);
                if let Some(mut material) = materials.get_mut(&aura_material.0) {
                    material.base_color.set_alpha(0.15);
                }
            } else {
                aura_transform.scale = Vec3::ZERO;
            }
        }
    }
}

/// Despawns the composite when leaving gameplay so re-entry re-creates it.
/// Bevy 0.19's `EntityCommands::despawn` is recursive by default.
fn cleanup_dragon_parts(mut commands: Commands, rigs: Query<Entity, With<DragonRig>>) {
    for entity in rigs.iter() {
        commands.entity(entity).despawn();
    }
}
