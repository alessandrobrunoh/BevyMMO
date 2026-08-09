//! Client-side point-and-click movement: click selection and command rings.
//!
//! The predicted move system that mirrors the server's authoritative stepping
//! lives in `bevymmo_presentation::player_movement`, because it depends on
//! `ObservedCasts` (which the presentation crate owns). This module hosts only
//! the pure client systems that translate mouse input into a move target and
//! render the click-feedback rings.

use bevy::color::Color;
use bevy::math::primitives::InfinitePlane3d;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use bevymmo_shared::movement::{resolve_ray_to_ground, ClientSurfaceQuery, MoveTarget};
use bevymmo_shared::network::mode;
use bevymmo_shared::network::protocol::{Channel2, MoveCommand};
use lightyear::prelude::MessageSender;

use crate::network::types::ConnectedClient;

const INDICATOR_DURATION: f32 = 0.55;

/// While the right mouse button is held we re-broadcast the move target so the
/// player keeps following the cursor. Sending every frame would flood the
/// server, so we cap the send rate at ~20 Hz.
const HELD_MOVE_SEND_INTERVAL: f32 = 0.05;

pub struct PlayerMovementPlugin;

#[derive(Component)]
struct ClickIndicator {
    elapsed: f32,
    delay: f32,
}

impl Plugin for PlayerMovementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MoveTarget>();
        app.init_resource::<ClientSurfaceQuery>();
        app.add_systems(
            Update,
            (select_move_target, animate_click_indicators).run_if(mode::has_client),
        );
    }
}

/// Reads left click on terrain and stores the point to send to the server.
fn select_move_target(
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    time: Res<Time>,
    mut send_cooldown: Local<f32>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut move_target: ResMut<MoveTarget>,
    surface_query: Res<ClientSurfaceQuery>,
    mut move_senders: Query<&mut MessageSender<MoveCommand>, With<ConnectedClient>>,
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let Some(mouse_buttons) = mouse_buttons else {
        return;
    };

    // We distinguish initial click (with visual indicator) from held key:
    // in the latter case we only update destination without spamming rings.
    let just_pressed = mouse_buttons.just_pressed(MouseButton::Right);
    let held = mouse_buttons.pressed(MouseButton::Right);
    if !held {
        *send_cooldown = 0.0;
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

    // Use height-aware ray-to-surface resolution when surface data is available
    let target = if let Some(surface_query) = surface_query.0.as_ref() {
        resolve_ray_to_ground(
            ray.origin,
            *ray.direction, // Convert Dir3 to Vec3
            surface_query,
            100.0, // max_distance
            0.5,   // step_size
        )
    } else {
        // Fallback to Y=0 plane when no surface data is available
        ray.plane_intersection_point(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))
            .map(|point| Vec3::new(point.x, 0.0, point.z))
    };

    let Some(target) = target else {
        return;
    };

    move_target.0 = Some(target);

    // Always forward the very first press immediately; while the button stays
    // held, forward the updated cursor target at a throttled rate so the
    // player keeps following the pointer without flooding the server.
    *send_cooldown -= time.delta_secs();
    if just_pressed || *send_cooldown <= 0.0 {
        if just_pressed {
            info!(
                "Client movement target set to ({:.2}, {:.2}, {:.2})",
                target.x, target.y, target.z
            );
        }
        for mut sender in &mut move_senders {
            sender.send::<Channel2>(MoveCommand { target });
        }
        *send_cooldown = HELD_MOVE_SEND_INTERVAL;
    }

    if !just_pressed {
        return;
    }

    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    spawn_click_indicator(&mut commands, &mut meshes, &mut materials, target);
}

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

/// Two rings expand and vanish quickly, like a MOBA command indicator.
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
