//! Client-side point-and-click movement: click selection and command rings.
//!
//! The predicted move system that mirrors the server's authoritative stepping
//! lives in `bevymmo_presentation::player_movement`, because it depends on
//! `ObservedCasts` (which the presentation crate owns). This module hosts only
//! the pure client systems that translate mouse input into a move target and
//! render the click-feedback rings.

use bevy::color::Color;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::movement::{resolve_click_to_ground, ClientSurfaceQuery, MoveTarget};
use crate::pointer::{hud_wants_pointer, PointerOnHud};
use bevymmo_network::network::mode;

const INDICATOR_DURATION: f32 = 0.55;

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
        app.init_resource::<PointerOnHud>();
        app.add_systems(
            Update,
            (select_move_target, animate_click_indicators).run_if(mode::has_client),
        );
    }
}

/// Reads left click on terrain and stores the point to send to the server.
///
/// `pub(crate)` so `crate::stdb::plugin::send_move_commands` can order
/// itself `.after()` this system: it now reads [`MoveTarget`] instead of
/// re-resolving the click itself, so it needs this system's write for the
/// frame to have already happened.
pub(crate) fn select_move_target(
    mouse_buttons: Option<Res<ButtonInput<MouseButton>>>,
    pointer_on_hud: Res<PointerOnHud>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &Transform), With<Camera3d>>,
    mut move_target: ResMut<MoveTarget>,
    surface_query: Res<ClientSurfaceQuery>,
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
    let Some(mouse_buttons) = mouse_buttons else {
        return;
    };
    if hud_wants_pointer(&pointer_on_hud) {
        return;
    }

    // We distinguish initial click (with visual indicator) from held key:
    // in the latter case we only update destination without spamming rings.
    let just_pressed = mouse_buttons.just_pressed(MouseButton::Right);
    let held = mouse_buttons.pressed(MouseButton::Right);
    if !held {
        return;
    }

    let Some(target) = resolve_click_to_ground(&windows, &cameras, &surface_query, 300.0) else {
        return;
    };

    move_target.0 = Some(target);

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
