//! Movimento punta-e-clicca del player e indicatore visivo del comando.

use bevy::math::primitives::InfinitePlane3d;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use lightyear::prelude::client::input::InputSystems;
use lightyear::prelude::input::native::{ActionState, InputMarker};
use lightyear::prelude::*;

use crate::network::mode;
use crate::network::protocol::{Inputs, LookDirection, Position};
use crate::plugins::entity::components::EntityState;
use crate::plugins::entity::player::Player;
use crate::stats::components::MovementStats;

const ARRIVAL_DISTANCE: f32 = 0.05;
const INDICATOR_DURATION: f32 = 0.55;

pub struct PlayerMovementPlugin;

#[derive(Resource, Default)]
struct MoveTarget(Option<Vec3>);

#[cfg(feature = "client")]
#[derive(Component)]
struct ClickIndicator {
    elapsed: f32,
    delay: f32,
}

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MoveTarget>();
        app.add_systems(
            FixedPreUpdate,
            buffer_move_input
                .in_set(InputSystems::WriteClientInputs)
                .run_if(mode::has_client),
        );
        app.add_systems(
            FixedUpdate,
            (
                server_move_to_target.run_if(mode::has_server),
                predict_move_to_target.run_if(mode::has_client),
            ),
        );
        #[cfg(feature = "client")]
        app.add_systems(
            Update,
            (select_move_target, animate_click_indicators).run_if(mode::has_client),
        );
    }
}

/// Legge il click sinistro sul terreno e salva il punto da inviare al server.
#[cfg(feature = "client")]
fn select_move_target(
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut move_target: ResMut<MoveTarget>,
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let Some(mouse_buttons) = mouse_buttons else {
        return;
    };
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let Some((camera, camera_transform)) = cameras.iter().next() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };
    let Some(target) = ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
    else {
        return;
    };

    let target = Vec3::new(target.x, 0.0, target.z);
    move_target.0 = Some(target);

    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    spawn_click_indicator(&mut commands, &mut meshes, &mut materials, target);
}

/// Scrive lo stesso input per ogni tick, così il server mantiene il comando fino all'arrivo.
fn buffer_move_input(
    move_target: Res<MoveTarget>,
    mut players: Query<&mut ActionState<Inputs>, With<InputMarker<Inputs>>>,
) {
    let input = move_target.0.map(Inputs::MoveTo).unwrap_or(Inputs::Stop);

    for mut player in &mut players {
        player.0 = input.clone();
    }
}

/// Movimento autoritativo: il server aggiorna soltanto i Player verso il target ricevuto.
fn server_move_to_target(
    mut players: Query<
        (
            &mut Position,
            &ActionState<Inputs>,
            &MovementStats,
            &mut LookDirection,
            &mut EntityState,
        ),
        With<Player>,
    >,
) {
    for (position, input, stats, look_direction, state) in &mut players {
        move_towards_target(position, look_direction, &input.0, stats.speed, state);
    }
}

/// Stesso calcolo sul Player predetto, per una risposta immediata al click.
fn predict_move_to_target(
    synced_client: Query<(), (With<Client>, With<IsSynced<()>>)>,
    mut players: Query<
        (
            &mut Position,
            &ActionState<Inputs>,
            &MovementStats,
            &mut LookDirection,
            &mut EntityState,
        ),
        (With<Player>, With<Predicted>),
    >,
) {
    if synced_client.is_empty() {
        return;
    }

    for (position, input, stats, look_direction, state) in &mut players {
        move_towards_target(position, look_direction, &input.0, stats.speed, state);
    }
}

fn move_towards_target(
    mut position: Mut<Position>,
    mut look_direction: Mut<LookDirection>,
    input: &Inputs,
    speed: f32,
    mut state: Mut<EntityState>,
) {
    if state.is_dead() {
        return;
    }

    let Inputs::MoveTo(target) = input else {
        *state = EntityState::Idle;
        return;
    };

    let offset = *target - position.0;
    let distance = offset.length();
    if distance > 0.001 {
        look_direction.0 = (offset / distance).normalize_or_zero();
    }
    if distance <= ARRIVAL_DISTANCE {
        position.0 = *target;
        *state = EntityState::Idle;
        return;
    }

    position.0 += offset / distance * speed.min(distance);
    *state = EntityState::Moving;
}

#[cfg(feature = "client")]
fn spawn_click_indicator(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    target: Vec3,
) {
    let mesh = meshes.add(Torus::new(0.62, 0.72));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.7, 1.0),
        emissive: LinearRgba::rgb(0.05, 0.35, 0.8),
        ..default()
    });

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(target + Vec3::Y * 0.03)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::splat(0.35)),
        ClickIndicator {
            elapsed: 0.0,
            delay: 0.0,
        },
    ));

    commands.spawn((
        Mesh3d(meshes.add(Torus::new(0.62, 0.72))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.65, 0.9, 1.0),
            emissive: LinearRgba::rgb(0.3, 0.6, 1.0),
            ..default()
        })),
        Transform::from_translation(target + Vec3::Y * 0.04)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::splat(0.2)),
        ClickIndicator {
            elapsed: 0.0,
            delay: 0.12,
        },
    ));
}

/// Due anelli si espandono e scompaiono rapidamente, come un indicatore di comando MOBA.
#[cfg(feature = "client")]
fn animate_click_indicators(
    time: Res<Time>,
    mut commands: Commands,
    mut indicators: Query<(Entity, &mut Transform, &mut ClickIndicator)>,
) {
    for (entity, mut transform, mut indicator) in &mut indicators {
        indicator.elapsed += time.delta_secs();
        if indicator.elapsed >= INDICATOR_DURATION {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = ((indicator.elapsed - indicator.delay)
            / (INDICATOR_DURATION - indicator.delay))
            .clamp(0.0, 1.0);
        transform.scale = Vec3::splat(0.2 + progress * 1.1);
    }
}
