//! Orbit camera for the editor viewport.
//!
//! Behaves like an iso-friendly camera: fixed pitch range, yaw free,
//! pan via WASD / middle drag, orbit via right drag, distance via scroll.
//! `F` snaps the focus onto the current selection.

use bevy::gizmos::transform_gizmo::TransformGizmoCamera;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

use crate::state::EditorState;

const MIN_PITCH: f32 = 0.2;
const MAX_PITCH: f32 = 1.3;
const MIN_DISTANCE: f32 = 5.0;
const MAX_DISTANCE: f32 = 150.0;
const PAN_SPEED: f32 = 0.05;
const KEYBOARD_PAN_SPEED: f32 = 6.0;
const KEYBOARD_PAN_SPEED_FAST: f32 = 18.0;

#[derive(Component)]
pub struct EditorCamera;

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 20.0, 25.0).looking_at(Vec3::ZERO, Vec3::Y),
        EditorCamera,
        TransformGizmoCamera,
    ));
}

/// Keyboard navigation: WASD pans the camera focus on the ground plane
/// (camera-relative), `F` focuses the selection. Runs even while the pointer
/// hovers an egui panel, but not while egui is typing.
pub fn keyboard_pan(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    transforms: Query<&Transform>,
    mut state: ResMut<EditorState>,
) {
    if keyboard.just_pressed(KeyCode::KeyF) {
        if let Some(entity) = state.selected {
            if let Ok(transform) = transforms.get(entity) {
                state.camera_focus = transform.translation;
            }
        }
    }

    let mut pan = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        pan.z += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        pan.z -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        pan.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        pan.x += 1.0;
    }
    if pan == Vec3::ZERO {
        return;
    }

    let speed = if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
        KEYBOARD_PAN_SPEED_FAST
    } else {
        KEYBOARD_PAN_SPEED
    };
    let world_delta = Quat::from_rotation_y(state.camera_yaw) * pan.normalize();
    state.camera_focus += world_delta * speed * time.delta_secs();
}

pub fn orbit_camera(
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut q: Query<&mut Transform, With<EditorCamera>>,
    mut state: ResMut<EditorState>,
) {
    let Ok(mut transform) = q.single_mut() else {
        return;
    };

    // Pan with middle mouse drag.
    if mouse.pressed(MouseButton::Middle) {
        for ev in motion.read() {
            let right = transform.rotation * Vec3::X;
            let forward = transform.rotation * Vec3::Z;
            let delta = right * (-ev.delta.x * PAN_SPEED) + forward * (ev.delta.y * PAN_SPEED);
            state.camera_focus += delta;
        }
        return;
    }

    // Orbit with right mouse drag.
    if mouse.pressed(MouseButton::Right) {
        for ev in motion.read() {
            state.camera_yaw -= ev.delta.x * 0.005;
            state.camera_pitch =
                (state.camera_pitch - ev.delta.y * 0.005).clamp(MIN_PITCH, MAX_PITCH);
        }
    }

    // Dolly with scroll.
    for ev in wheel.read() {
        let zoom = if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight)
        {
            ev.y * 5.0
        } else {
            ev.y * 2.0
        };
        state.camera_distance = (state.camera_distance - zoom).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    // Apply orbit transform.
    let distance = state.camera_distance;
    let focus = state.camera_focus;
    let yaw = state.camera_yaw;
    let pitch = state.camera_pitch;

    let dir = Vec3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    );
    let pos = focus + dir * distance;
    transform.translation = pos;
    transform.look_at(focus, Vec3::Y);
}
