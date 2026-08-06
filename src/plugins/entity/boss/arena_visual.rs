//! Client visualization of the boss arena trigger ring.
//!
//! While the boss is dormant (`BossArena.is_engaged == false`) the client draws
//! a pulsing red torus on the ground at the boss position. The moment the
//! server flips `is_engaged` to true (replicated), the ring fades out and is
//! despawned. This reads the replicated `BossArena` component only; no gameplay
//! logic lives here.

use std::collections::HashMap;

use bevy::color::Color;
use bevy::prelude::*;

use super::components::{Boss, BossArena};
use crate::game_state::{GameScreen, Screen};
use crate::network::mode::has_client;

/// Marker for the arena ring visual entity. Carries the boss it belongs to
/// so we can detect engage without relying on the `Parent` hierarchy API.
#[derive(Component)]
struct BossArenaRingVisual {
    boss: Entity,
}

/// Seconds the ring takes to fade out once the encounter engages.
const FADE_OUT_SECONDS: f32 = 0.6;

/// Tracks elapsed fade time; `is_fading` arms the despawn animation.
#[derive(Component, Default)]
struct RingFade {
    elapsed_seconds: f32,
    is_fading: bool,
}

pub fn client_arena_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            manage_arena_rings,
            animate_arena_rings.after(manage_arena_rings),
        )
            .run_if(has_client)
            .run_if(in_gameplay),
    )
    .add_systems(
        Update,
        cleanup_arena_rings
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

/// Ensures exactly one ring child exists per dormant boss, and arms the fade
/// once the encounter engages.
///
/// The ring is parented to the boss so it tracks the replicated `Position`
/// automatically.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(Update, manage_arena_rings.run_if(has_client));
/// ```
fn manage_arena_rings(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    bosses: Query<(Entity, &BossArena, Option<&Children>), With<Boss>>,
    rings: Query<(Entity, &BossArenaRingVisual, &RingFade)>,
) {
    let boss_states: HashMap<Entity, bool> = bosses
        .iter()
        .map(|(entity, arena, _)| (entity, arena.is_engaged))
        .collect();

    // Arm the fade for any ring whose boss is engaged (or whose boss is gone).
    for (ring, visual, fade) in rings.iter() {
        if fade.is_fading {
            continue;
        }
        let still_dormant = boss_states
            .get(&visual.boss)
            .is_some_and(|engaged| !*engaged);
        if !still_dormant {
            commands.entity(ring).insert(RingFade {
                elapsed_seconds: 0.0,
                is_fading: true,
            });
        }
    }

    // Spawn rings for dormant bosses that don't have one yet.
    for (boss, arena, children) in bosses.iter() {
        if arena.is_engaged {
            continue;
        }
        let already_has_ring = children.is_some_and(|kids| {
            kids.iter().any(|child| {
                rings
                    .get(child)
                    .is_ok_and(|(_, visual, _)| visual.boss == boss)
            })
        });
        if already_has_ring {
            continue;
        }

        let ring_radius = arena.radius.max(0.0);
        let mesh = meshes.add(Torus {
            major_radius: ring_radius,
            minor_radius: 0.3,
        });
        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.95, 0.10, 0.10, 0.5),
            emissive: LinearRgba::rgb(0.60, 0.05, 0.05),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });

        commands.entity(boss).with_children(|parent| {
            parent.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                // Lay the torus flat on the ground; local offset is in the
                // boss's frame (boss is the parent).
                Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
                    .with_translation(Vec3::new(0.0, 0.1, 0.0)),
                BossArenaRingVisual { boss },
                RingFade::default(),
            ));
        });
    }
}

/// Pulses the ring scale while dormant and fades the material alpha out once
/// engaged, then despawns.
///
/// Material mutation goes through `Assets<StandardMaterial>::get_mut` because
/// the entity only holds a `Handle` to the shared asset.
fn animate_arena_rings(
    time: Res<Time>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rings: Query<
        (
            Entity,
            &mut Transform,
            &MeshMaterial3d<StandardMaterial>,
            &mut RingFade,
        ),
        With<BossArenaRingVisual>,
    >,
) {
    let t = time.elapsed_secs();
    for (entity, mut transform, material_handle, mut fade) in rings.iter_mut() {
        if fade.is_fading {
            fade.elapsed_seconds += time.delta_secs();
            let progress = (fade.elapsed_seconds / FADE_OUT_SECONDS).clamp(0.0, 1.0);
            if let Some(mut material) = materials.get_mut(&material_handle.0) {
                material.base_color.set_alpha(0.5 * (1.0 - progress));
            }
            if progress >= 1.0 {
                commands.entity(entity).despawn();
            }
            continue;
        }

        // Dormant: gentle scale pulse (alpha stays constant).
        let pulse = 1.0 + (t * 3.0).sin() * 0.02;
        transform.scale = Vec3::new(pulse, pulse, 1.0);
    }
}

/// Despawns all ring visuals when leaving gameplay (mirrors the spell-visual
/// cleanup pattern so re-entering the scene re-creates them cleanly).
fn cleanup_arena_rings(mut commands: Commands, rings: Query<Entity, With<BossArenaRingVisual>>) {
    for entity in rings.iter() {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::protocol::Position;

    fn minimal_app() -> App {
        let mut app = App::new();
        app.init_resource::<GameScreen>();
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        app.init_resource::<Time>();
        app.add_systems(Update, (manage_arena_rings, animate_arena_rings));
        app
    }

    #[test]
    fn ring_is_spawned_for_a_dormant_boss() {
        let mut app = minimal_app();

        let boss = app
            .world_mut()
            .spawn((
                Boss,
                Position(Vec3::ZERO),
                BossArena {
                    center: Vec3::ZERO,
                    radius: 8.0,
                    is_engaged: false,
                },
            ))
            .id();

        app.update();

        let children = app.world().entity(boss).get::<Children>().unwrap();
        assert_eq!(children.len(), 1);
        assert!(app
            .world()
            .entity(children[0])
            .contains::<BossArenaRingVisual>());
    }

    #[test]
    fn ring_is_not_spawned_for_an_engaged_boss() {
        let mut app = minimal_app();

        let boss = app
            .world_mut()
            .spawn((
                Boss,
                Position(Vec3::ZERO),
                BossArena {
                    center: Vec3::ZERO,
                    radius: 8.0,
                    is_engaged: true,
                },
            ))
            .id();

        app.update();

        assert!(app.world().entity(boss).get::<Children>().is_none());
    }
}
