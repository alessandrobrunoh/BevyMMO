//! Left-click a harvestable node to start a server-authoritative gather.

use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};

use crate::app_state::{PauseOverlay, Screen, in_gameplay, in_unpaused_gameplay};
use crate::local_player::LocalPlayer;
use crate::movement::cursor_ray;
use crate::pointer::{PointerOnHud, hud_wants_pointer};
use crate::server_feed::ServerNotice;
use crate::stdb::{StdbConnection, commands};
use bevymmo_gameplay::entity::components::{EntityKind, GameEntity};
use bevymmo_gameplay::gathering::{ActiveGather, Harvestable, in_interact_range};
use bevymmo_gameplay::placeables::{KindId, PlaceableRegistry};
use bevymmo_network::world_components::{NetworkEntityId, Position};
use bevymmo_world::CollisionShape;

/// Same click radius as greeter / market NPC pick (`NPC_SELECT_RADIUS`).
const GATHER_SELECT_RADIUS: f32 = 1.2;
const DEFAULT_INTERACT_RANGE: f32 = 6.0;
const DEFAULT_PICK_HEIGHT: f32 = 5.5;
const PICK_HEIGHT_STEPS: u32 = 4;
const DEPLETED_MESSAGE: &str = "Questa risorsa è già stata completamente raccolta";

/// World click → `start_gather` / `stop_gather`, plus hover cursor.
pub struct GatheringPlugin;

impl Plugin for GatheringPlugin {
    fn build(&self, app: &mut App) {
        crate::pointer::PointerPlugin::ensure(app);
        app.add_systems(Update, click_resource_node.run_if(in_unpaused_gameplay))
            .add_systems(Update, hover_gather_cursor.run_if(in_gameplay))
            .add_systems(OnExit(Screen::InGame), reset_gather_cursor);
    }
}

/// What a left-click on the world would do to gathering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatherClick {
    /// Closest harvestable under the cursor, if any: `(node_id, current_pieces)`.
    pub hit_node: Option<(u64, u32)>,
    /// Local player currently has [`ActiveGather`].
    pub already_gathering: bool,
}

/// Action implied by a gather click. Hitting another node goes through `Start`
/// and replaces the session server-side — do not send `Stop` first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatherClickAction {
    Start(u64),
    Stop,
    DepletedNotice,
    None,
}

/// Maps a world click onto a gather command.
pub fn gather_click_action(click: GatherClick) -> GatherClickAction {
    match click.hit_node {
        Some((node_id, pieces)) if pieces > 0 => GatherClickAction::Start(node_id),
        Some(_) => GatherClickAction::DepletedNotice,
        None if click.already_gathering => GatherClickAction::Stop,
        None => GatherClickAction::None,
    }
}

/// Cursor while the pointer hovers a harvestable that is in interact range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatherHoverCursor {
    Pointer,
    NotAllowed,
    Default,
}

/// `hit_in_range` is `Some(pieces)` only when the pointer is over a harvestable
/// and the local player is within that node's interact range.
pub fn gather_hover_cursor(hit_in_range: Option<u32>) -> GatherHoverCursor {
    match hit_in_range {
        Some(pieces) if pieces > 0 => GatherHoverCursor::Pointer,
        Some(_) => GatherHoverCursor::NotAllowed,
        None => GatherHoverCursor::Default,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct HarvestablePick {
    node_id: u64,
    pieces: u32,
    position: Vec3,
}

/// Perpendicular distance from `point` to the camera ray. Same helper the
/// greeter sidebar and market NPC click use.
fn point_to_ray_distance(point: Vec3, ray_origin: Vec3, ray_direction: Vec3) -> f32 {
    let to_point = point - ray_origin;
    let projection = to_point.dot(ray_direction);
    let closest_on_ray = ray_origin + ray_direction * projection.clamp(0.0, f32::MAX);
    point.distance(closest_on_ray)
}

/// Min distance from the ray to a vertical segment `[base, base + height]`.
///
/// Greeter/market test a point at the feet. A tree is 5 m tall, so a click on
/// the canopy would miss that point; sampling along the trunk keeps the same
/// radius and the same metric.
fn ray_to_trunk_distance(base: Vec3, height: f32, ray: Ray3d) -> f32 {
    let height = height.max(0.0);
    (0..=PICK_HEIGHT_STEPS)
        .map(|step| {
            let y = height * (step as f32 / PICK_HEIGHT_STEPS as f32);
            point_to_ray_distance(base + Vec3::Y * y, ray.origin, *ray.direction)
        })
        .fold(f32::MAX, f32::min)
}

fn pick_harvestable(
    ray: Ray3d,
    nodes: impl Iterator<Item = (Vec3, u64, u32, f32)>,
) -> Option<HarvestablePick> {
    let mut closest: Option<(u64, f32, u32, Vec3)> = None;
    for (position, node_id, pieces, height) in nodes {
        let distance = ray_to_trunk_distance(position, height, ray);
        if distance > GATHER_SELECT_RADIUS {
            continue;
        }
        if closest.is_none_or(|(_, best, _, _)| distance < best) {
            closest = Some((node_id, distance, pieces, position));
        }
    }
    closest.map(|(node_id, _, pieces, position)| HarvestablePick {
        node_id,
        pieces,
        position,
    })
}

fn click_resource_node(
    mouse: Res<ButtonInput<MouseButton>>,
    pointer_on_hud: Res<PointerOnHud>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &Transform), With<Camera3d>>,
    conn: Option<Res<StdbConnection>>,
    placeables: Option<Res<PlaceableRegistry>>,
    nodes: Query<
        (&Position, &NetworkEntityId, &Harvestable, &EntityKind),
        (With<GameEntity>, Without<LocalPlayer>),
    >,
    local_gather: Query<&ActiveGather, With<LocalPlayer>>,
    mut notices: MessageWriter<ServerNotice>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    if hud_wants_pointer(&pointer_on_hud) {
        return;
    }
    let Some(conn) = conn else {
        return;
    };
    let Some(ray) = cursor_ray(&windows, &cameras) else {
        return;
    };

    let hit_node = pick_harvestable(
        ray,
        nodes
            .iter()
            .filter_map(|(position, network_id, harvestable, kind)| {
                (*kind == EntityKind::Resource).then_some((
                    position.0,
                    network_id.0,
                    harvestable.current_pieces,
                    pick_height_for(harvestable.kind_id.as_str(), placeables.as_deref()),
                ))
            }),
    )
    .map(|pick| (pick.node_id, pick.pieces));
    let already_gathering = local_gather.iter().next().is_some();

    match gather_click_action(GatherClick {
        hit_node,
        already_gathering,
    }) {
        GatherClickAction::Start(node_id) => {
            if let Err(err) = commands::start_gather(&conn, node_id) {
                error!("start_gather failed: {err}");
            }
        }
        GatherClickAction::Stop => {
            if let Err(err) = commands::stop_gather(&conn) {
                error!("stop_gather failed: {err}");
            }
        }
        GatherClickAction::DepletedNotice => {
            notices.write(ServerNotice::error(DEPLETED_MESSAGE));
        }
        GatherClickAction::None => {}
    }
}

fn hover_gather_cursor(
    mut commands: Commands,
    pointer_on_hud: Res<PointerOnHud>,
    pause: Option<Res<State<PauseOverlay>>>,
    windows: Query<(Entity, &Window, Option<&CursorIcon>), With<PrimaryWindow>>,
    cameras: Query<(&Camera, &Transform), With<Camera3d>>,
    placeables: Option<Res<PlaceableRegistry>>,
    player: Query<&Position, With<LocalPlayer>>,
    nodes: Query<
        (&Position, &NetworkEntityId, &Harvestable, &EntityKind),
        (With<GameEntity>, Without<LocalPlayer>),
    >,
) {
    let Ok((window_entity, window, current_icon)) = windows.single() else {
        return;
    };

    let paused = pause.is_some_and(|pause| *pause.get() == PauseOverlay::On);
    let next = if paused || hud_wants_pointer(&pointer_on_hud) {
        GatherHoverCursor::Default
    } else {
        gather_hover_cursor(hover_pieces_in_range(
            window,
            &cameras,
            placeables.as_deref(),
            &player,
            &nodes,
        ))
    };

    apply_gather_cursor(&mut commands, window_entity, current_icon, next);
}

fn hover_pieces_in_range(
    window: &Window,
    cameras: &Query<(&Camera, &Transform), With<Camera3d>>,
    placeables: Option<&PlaceableRegistry>,
    player: &Query<&Position, With<LocalPlayer>>,
    nodes: &Query<
        (&Position, &NetworkEntityId, &Harvestable, &EntityKind),
        (With<GameEntity>, Without<LocalPlayer>),
    >,
) -> Option<u32> {
    let cursor_position = window.cursor_position()?;
    let (camera, camera_transform) = cameras.iter().next()?;
    let view = GlobalTransform::from(*camera_transform);
    let ray = camera.viewport_to_world(&view, cursor_position).ok()?;
    let pick = pick_harvestable(
        ray,
        nodes
            .iter()
            .filter_map(|(position, network_id, harvestable, kind)| {
                (*kind == EntityKind::Resource).then_some((
                    position.0,
                    network_id.0,
                    harvestable.current_pieces,
                    pick_height_for(harvestable.kind_id.as_str(), placeables),
                ))
            }),
    )?;
    let player_pos = player.single().ok()?;
    let kind_id = nodes.iter().find_map(|(_, network_id, harvestable, _)| {
        (network_id.0 == pick.node_id).then_some(harvestable.kind_id.as_str())
    })?;
    let range = interact_range_for(kind_id, placeables);
    in_interact_range(
        player_pos.0.x,
        player_pos.0.z,
        pick.position.x,
        pick.position.z,
        range,
    )
    .then_some(pick.pieces)
}

fn interact_range_for(kind_id: &str, registry: Option<&PlaceableRegistry>) -> f32 {
    registry
        .and_then(|registry| registry.resources.get(&KindId::new(kind_id.to_owned())))
        .map(|definition| definition.resource_config().interact_range)
        .unwrap_or(DEFAULT_INTERACT_RANGE)
}

fn pick_height_for(kind_id: &str, registry: Option<&PlaceableRegistry>) -> f32 {
    registry
        .and_then(|registry| registry.resources.get(&KindId::new(kind_id.to_owned())))
        .and_then(|definition| definition.defaults().collision)
        .map(|shape| match shape {
            CollisionShape::Cylinder { height, .. } => height,
            CollisionShape::Box { half_extents } => half_extents[1] * 2.0,
            CollisionShape::Sphere { radius } => radius * 2.0,
        })
        .filter(|height| *height > 0.0)
        .unwrap_or(DEFAULT_PICK_HEIGHT)
}

fn apply_gather_cursor(
    commands: &mut Commands,
    window_entity: Entity,
    current: Option<&CursorIcon>,
    next: GatherHoverCursor,
) {
    match next {
        GatherHoverCursor::Default => {
            if current.is_some() {
                commands.entity(window_entity).remove::<CursorIcon>();
            }
        }
        GatherHoverCursor::Pointer => {
            let icon = CursorIcon::from(SystemCursorIcon::Pointer);
            if current != Some(&icon) {
                commands.entity(window_entity).insert(icon);
            }
        }
        GatherHoverCursor::NotAllowed => {
            let icon = CursorIcon::from(SystemCursorIcon::NotAllowed);
            if current != Some(&icon) {
                commands.entity(window_entity).insert(icon);
            }
        }
    }
}

fn reset_gather_cursor(mut commands: Commands, windows: Query<Entity, With<PrimaryWindow>>) {
    if let Ok(window) = windows.single() {
        commands.entity(window).remove::<CursorIcon>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miss_while_gathering_stops() {
        assert_eq!(
            gather_click_action(GatherClick {
                hit_node: None,
                already_gathering: true,
            }),
            GatherClickAction::Stop
        );
    }

    #[test]
    fn miss_while_idle_does_nothing() {
        assert_eq!(
            gather_click_action(GatherClick {
                hit_node: None,
                already_gathering: false,
            }),
            GatherClickAction::None
        );
    }

    #[test]
    fn hit_with_pieces_starts_gather() {
        assert_eq!(
            gather_click_action(GatherClick {
                hit_node: Some((7, 3)),
                already_gathering: false,
            }),
            GatherClickAction::Start(7)
        );
    }

    #[test]
    fn hit_empty_node_shows_depleted_notice() {
        assert_eq!(
            gather_click_action(GatherClick {
                hit_node: Some((7, 0)),
                already_gathering: true,
            }),
            GatherClickAction::DepletedNotice
        );
    }

    #[test]
    fn hit_other_node_starts_without_an_explicit_stop() {
        assert_eq!(
            gather_click_action(GatherClick {
                hit_node: Some((9, 2)),
                already_gathering: true,
            }),
            GatherClickAction::Start(9)
        );
    }

    #[test]
    fn hover_in_range_with_pieces_is_a_hand() {
        assert_eq!(gather_hover_cursor(Some(4)), GatherHoverCursor::Pointer);
    }

    #[test]
    fn hover_in_range_empty_is_not_allowed() {
        assert_eq!(gather_hover_cursor(Some(0)), GatherHoverCursor::NotAllowed);
    }

    #[test]
    fn hover_out_of_range_or_miss_is_default() {
        assert_eq!(gather_hover_cursor(None), GatherHoverCursor::Default);
    }

    fn downward_ray_through(point: Vec3) -> Ray3d {
        Ray3d::new(point + Vec3::Y * 8.0, Dir3::NEG_Y)
    }

    #[test]
    fn canopy_click_selects_the_tree_like_an_npc_click() {
        let base = Vec3::new(4.0, 1.0, -2.0);
        let ray = downward_ray_through(base + Vec3::Y * 4.5);
        let pick = pick_harvestable(ray, [(base, 7, 12, 5.5)].into_iter());
        assert_eq!(pick.map(|pick| pick.node_id), Some(7));
        assert_eq!(pick.map(|pick| pick.pieces), Some(12));
    }

    #[test]
    fn click_beside_the_tree_does_not_select_it() {
        let base = Vec3::ZERO;
        let ray = downward_ray_through(base + Vec3::X * 4.0);
        assert!(pick_harvestable(ray, [(base, 7, 12, 5.5)].into_iter()).is_none());
    }

    #[test]
    fn tighter_aim_wins_between_two_trees() {
        let ray = downward_ray_through(Vec3::ZERO);
        let pick = pick_harvestable(
            ray,
            [(Vec3::X, 2, 1, 5.5), (Vec3::X * 0.2, 1, 1, 5.5)].into_iter(),
        );
        assert_eq!(pick.map(|pick| pick.node_id), Some(1));
    }
}
