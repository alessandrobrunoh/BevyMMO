//! Point-and-click player movement and visual command indicator.

#[cfg(feature = "client")]
use bevy::color::Color;
#[cfg(feature = "client")]
use bevy::math::primitives::InfinitePlane3d;
use bevy::prelude::*;
#[cfg(feature = "client")]
use bevy::window::PrimaryWindow;
use lightyear::prelude::client::input::InputSystems;
use lightyear::prelude::input::native::{ActionState, InputMarker};
use lightyear::prelude::*;

use crate::network::mode;
#[cfg(feature = "client")]
use crate::network::protocol::NetworkEntityId;
use crate::network::protocol::{Inputs, LookDirection, Position};
use crate::plugins::entity::components::EntityState;
use crate::plugins::entity::player::Player;
use crate::plugins::spells::{CastKind, CastProgress};
use crate::stats::components::MovementStats;
use crate::stats::events::StatField;
use crate::stats::modifiers::ActiveStatModifiers;
use crate::stats::systems::effective_value;
#[cfg(feature = "client")]
use crate::{plugins::spells::cast_bar::ObservedCasts, spells::swift::SwiftSpell};

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
        app.add_systems(FixedUpdate, server_move_to_target.run_if(mode::has_server));
        #[cfg(feature = "client")]
        app.add_systems(FixedUpdate, predict_move_to_target.run_if(mode::has_client));
        #[cfg(feature = "client")]
        app.add_systems(
            Update,
            (select_move_target, animate_click_indicators).run_if(mode::has_client),
        );
    }
}

/// Reads left click on terrain and stores the point to send to the server.
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

    // We distinguish initial click (with visual indicator) from held key:
    // in the latter case we only update destination without spamming rings.
    let just_pressed = mouse_buttons.just_pressed(MouseButton::Right);
    let held = mouse_buttons.pressed(MouseButton::Right);
    if !held {
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

    if !just_pressed {
        return;
    }

    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    spawn_click_indicator(&mut commands, &mut meshes, &mut materials, target);
}

/// Writes the same input for each tick, so the server retains the command until arrival.
fn buffer_move_input(
    move_target: Res<MoveTarget>,
    mut players: Query<&mut ActionState<Inputs>, With<InputMarker<Inputs>>>,
) {
    let input = move_target.0.map(Inputs::MoveTo).unwrap_or(Inputs::Stop);

    for mut player in &mut players {
        player.0 = input.clone();
    }
}

/// Authoritative movement: the server updates only Players towards the received target.
fn server_move_to_target(
    mut players: Query<
        (
            &mut Position,
            &ActionState<Inputs>,
            &MovementStats,
            &mut LookDirection,
            &mut EntityState,
            Option<&ActiveStatModifiers>,
            Option<&CastProgress>,
            Option<&crate::plugins::crowd_control::CrowdControlState>,
        ),
        With<Player>,
    >,
) {
    for (position, input, stats, look_direction, mut state, modifiers, cast, cc_state) in
        &mut players
    {
        if cc_state.map(|c| c.has_blocking_cc()).unwrap_or(false) {
            *state = EntityState::Idle;
            continue;
        }

        if should_block_movement_for_cast(cast) {
            *state = EntityState::Idle;
            continue;
        }

        let effective_speed = effective_movement_speed(stats.speed, modifiers);
        move_towards_target(position, look_direction, &input.0, effective_speed, state);
    }
}

/// Same calculation on predicted Player for immediate click response.
#[cfg(feature = "client")]
fn predict_move_to_target(
    synced_client: Query<(), (With<Client>, With<IsSynced<()>>)>,
    observed_casts: Option<Res<ObservedCasts>>,
    mut players: Query<
        (
            &mut Position,
            &ActionState<Inputs>,
            &MovementStats,
            &NetworkEntityId,
            &mut LookDirection,
            &mut EntityState,
            Option<&ActiveStatModifiers>,
            Option<&crate::plugins::crowd_control::CrowdControlState>,
        ),
        (With<Player>, With<Predicted>),
    >,
) {
    if synced_client.is_empty() {
        return;
    }

    for (position, input, stats, network_id, look_direction, mut state, modifiers, cc_state) in
        &mut players
    {
        if cc_state.map(|c| c.has_blocking_cc()).unwrap_or(false) {
            *state = EntityState::Idle;
            continue;
        }

        let effective_speed = predicted_effective_speed(
            stats.speed,
            modifiers,
            network_id,
            observed_casts.as_deref(),
        );
        move_towards_target(position, look_direction, &input.0, effective_speed, state);
    }
}

/// Calculates effective speed applying all active `Speed` modifiers
/// on the entity. Without modifiers, returns unchanged base value.
/// Calculates movement speed after active stat modifiers.
///
/// This is shared by movement and the stats UI so the value displayed to the
/// player matches the speed used by gameplay.
///
/// # Example
/// ```rust,ignore
/// let speed = effective_movement_speed(base_speed, modifiers);
/// ```
pub fn effective_movement_speed(base_speed: f32, modifiers: Option<&ActiveStatModifiers>) -> f32 {
    let Some(active) = modifiers else {
        return base_speed;
    };
    effective_value(StatField::Speed, base_speed, &active.modifiers)
}

/// Returns true when a cast state must freeze point-and-click movement.
///
/// CastTime always blocks movement. Channeling blocks only for policies that
/// interrupt on movement; Swift uses `AllowMovement`, so it keeps running.
///
/// # Example
/// ```rust,ignore
/// if should_block_movement_for_cast(cast) { return; }
/// ```
fn should_block_movement_for_cast(cast: Option<&CastProgress>) -> bool {
    let Some(cast) = cast else {
        return false;
    };
    match cast.kind {
        CastKind::CastTime => true,
        CastKind::Channeling => {
            cast.channel_movement == crate::plugins::spells::ChannelMovementPolicy::InterruptOnMove
        }
        CastKind::Instant => false,
    }
}

#[cfg(feature = "client")]
/// Mirrors the server-authoritative Swift speed boost for predicted movement.
///
/// The canonical buff still lives on the server as a stat modifier. The client
/// uses observed channel progress only to keep local prediction responsive while
/// holding `F`, avoiding visible rubber-banding.
///
/// # Example
/// ```rust,ignore
/// let speed = predicted_effective_speed(base_speed, modifiers, network_id, observed_casts);
/// ```
fn predicted_effective_speed(
    base_speed: f32,
    modifiers: Option<&ActiveStatModifiers>,
    network_id: &NetworkEntityId,
    observed_casts: Option<&ObservedCasts>,
) -> f32 {
    let server_speed = effective_movement_speed(base_speed, modifiers);
    let Some(observed_casts) = observed_casts else {
        return server_speed;
    };
    let Some(cast) = observed_casts.0.get(&network_id.0) else {
        return server_speed;
    };
    if cast.spell_id != SwiftSpell::ID {
        return server_speed;
    }

    server_speed * SwiftSpell::SPEED_MULTIPLIER
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

/// Two rings expand and vanish quickly, like a MOBA command indicator.
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

