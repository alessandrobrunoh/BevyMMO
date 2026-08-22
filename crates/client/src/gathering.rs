//! Left-click a harvestable node to start a server-authoritative gather.

use bevy::prelude::*;
use bevy::window::{CursorIcon, PrimaryWindow, SystemCursorIcon};

use crate::app_state::{in_gameplay, PauseOverlay, Screen};
use crate::local_player::LocalPlayer;
use crate::pointer::{hud_wants_pointer, PointerOnHud};
use bevymmo_gameplay::gathering::{in_interact_range, Harvestable};
use bevymmo_gameplay::placeables::{KindId, PlaceableRegistry};
use bevymmo_network::world_components::{NetworkEntityId, Position};
use bevymmo_world::CollisionShape;

/// Half-width of the click volume around the trunk. Wider than the greeter's
/// 1.2 m sphere because the oak canopy is the thing you actually click, and
/// `tree_oak_medium`'s crown reaches roughly 4 m out from the trunk.
const GATHER_PICK_HALF_XZ: f32 = 3.0;

/// The authored collider describes the trunk a character walks into (the oak's
/// is 0.4 m across and 5.5 m tall), not the silhouette on screen: the same oak
/// is 11.9 m tall, and everything above 5.5 m is canopy. Clicks land on the
/// canopy, so the pick volume covers twice the collider's height.
const PICK_HEIGHT_TO_COLLIDER: f32 = 2.0;
const DEFAULT_INTERACT_RANGE: f32 = 6.0;
const DEFAULT_PICK_HEIGHT: f32 = 5.5;
pub const TOO_FAR_MESSAGE: &str = "Avvicinati all'albero per raccogliere";

/// World click → `start_gather` / `stop_gather`, plus hover cursor.
pub struct GatheringPlugin;

impl Plugin for GatheringPlugin {
    fn build(&self, app: &mut App) {
        crate::pointer::PointerPlugin::ensure(app);
        app.add_systems(Update, hover_gather_cursor.run_if(in_gameplay))
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
pub struct HarvestablePick {
    pub node_id: u64,
    pub pieces: u32,
    pub position: Vec3,
}

/// Ray vs the click volume of a harvestable: a tall AABB around the trunk so
/// clicking the canopy counts, the same way greeter/market count a click near
/// the NPC's origin.
pub fn pick_harvestable(
    ray: Ray3d,
    nodes: impl Iterator<Item = (Vec3, u64, u32, f32)>,
) -> Option<HarvestablePick> {
    let mut closest: Option<(u64, f32, u32, Vec3)> = None;
    for (position, node_id, pieces, height) in nodes {
        let Some(t) = ray_hits_gather_aabb(ray, position, height) else {
            continue;
        };
        if closest.is_none_or(|(_, best, _, _)| t < best) {
            closest = Some((node_id, t, pieces, position));
        }
    }
    closest.map(|(node_id, _, pieces, position)| HarvestablePick {
        node_id,
        pieces,
        position,
    })
}

fn ray_hits_gather_aabb(ray: Ray3d, base: Vec3, height: f32) -> Option<f32> {
    let height = height.max(1.0);
    let min = Vec3::new(
        base.x - GATHER_PICK_HALF_XZ,
        base.y - 0.5,
        base.z - GATHER_PICK_HALF_XZ,
    );
    let max = Vec3::new(
        base.x + GATHER_PICK_HALF_XZ,
        base.y + height,
        base.z + GATHER_PICK_HALF_XZ,
    );
    ray_aabb_t(ray, min, max)
}

fn ray_aabb_t(ray: Ray3d, min: Vec3, max: Vec3) -> Option<f32> {
    let origin = ray.origin;
    let dir = *ray.direction;
    let mut tmin = 0.0f32;
    let mut tmax = f32::MAX;
    for i in 0..3 {
        let d = dir[i];
        let o = origin[i];
        if d.abs() < 1e-8 {
            if o < min[i] || o > max[i] {
                return None;
            }
            continue;
        }
        let mut t1 = (min[i] - o) / d;
        let mut t2 = (max[i] - o) / d;
        if t1 > t2 {
            core::mem::swap(&mut t1, &mut t2);
        }
        tmin = tmin.max(t1);
        tmax = tmax.min(t2);
        if tmin > tmax {
            return None;
        }
    }
    if tmax < 0.0 {
        return None;
    }
    Some(tmin.max(0.0))
}

pub fn pick_height_for(kind_id: &str, registry: Option<&PlaceableRegistry>) -> f32 {
    registry
        .and_then(|registry| registry.resources.get(&KindId::new(kind_id.to_owned())))
        .and_then(|definition| definition.defaults().collision)
        .map(|shape| match shape {
            CollisionShape::Cylinder { height, .. } => height,
            CollisionShape::Box { half_extents } => half_extents[1] * 2.0,
            CollisionShape::Sphere { radius } => radius * 2.0,
        })
        .filter(|height| *height > 0.0)
        .map(|height| height * PICK_HEIGHT_TO_COLLIDER)
        .unwrap_or(DEFAULT_PICK_HEIGHT)
}

/// Closest harvestable the player can actually channel, ignoring the ray.
/// Used when the click hits the local nametag sitting on the trunk.
pub fn nearest_harvestable_in_range(
    player: Vec3,
    nodes: impl Iterator<Item = (Vec3, u64, u32, f32)>,
) -> Option<(u64, u32)> {
    nodes
        .filter(|(position, _, _, range)| {
            in_interact_range(player.x, player.z, position.x, position.z, *range)
        })
        .min_by(|a, b| {
            let da = (a.0.x - player.x).hypot(a.0.z - player.z);
            let db = (b.0.x - player.x).hypot(b.0.z - player.z);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, node_id, pieces, _)| (node_id, pieces))
}

pub fn interact_range_for(kind_id: &str, registry: Option<&PlaceableRegistry>) -> f32 {
    registry
        .and_then(|registry| registry.resources.get(&KindId::new(kind_id.to_owned())))
        .map(|definition| definition.resource_config().interact_range)
        .unwrap_or(DEFAULT_INTERACT_RANGE)
}

fn hover_gather_cursor(
    mut commands: Commands,
    pointer_on_hud: Res<PointerOnHud>,
    pause: Option<Res<State<PauseOverlay>>>,
    windows: Query<(Entity, &Window, Option<&CursorIcon>), With<PrimaryWindow>>,
    cameras: Query<(&Camera, &Transform), With<Camera3d>>,
    placeables: Option<Res<PlaceableRegistry>>,
    player: Query<&Position, With<LocalPlayer>>,
    nodes: Query<(&Position, &NetworkEntityId, &Harvestable), Without<LocalPlayer>>,
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
    nodes: &Query<(&Position, &NetworkEntityId, &Harvestable), Without<LocalPlayer>>,
) -> Option<u32> {
    let cursor_position = window.cursor_position()?;
    let (camera, camera_transform) = cameras.iter().next()?;
    let view = GlobalTransform::from(*camera_transform);
    let ray = camera.viewport_to_world(&view, cursor_position).ok()?;
    let pick = pick_harvestable(
        ray,
        nodes.iter().map(|(position, network_id, harvestable)| {
            (
                position.0,
                network_id.0,
                harvestable.current_pieces,
                pick_height_for(harvestable.kind_id.as_str(), placeables),
            )
        }),
    )?;
    let player_pos = player.single().ok()?;
    let kind_id = nodes.iter().find_map(|(_, network_id, harvestable)| {
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
    fn standing_next_to_a_tree_selects_it_without_a_ray_hit() {
        let player = Vec3::ZERO;
        let next_to = nearest_harvestable_in_range(
            player,
            [
                (Vec3::new(2.0, 0.0, 0.0), 1, 10, 6.0),
                (Vec3::new(40.0, 0.0, 0.0), 2, 10, 6.0),
            ]
            .into_iter(),
        );
        assert_eq!(next_to, Some((1, 10)));
    }

    #[test]
    fn nearer_tree_along_the_ray_wins() {
        let ray = Ray3d::new(Vec3::new(0.0, 2.0, -4.0), Dir3::Z);
        let pick = pick_harvestable(
            ray,
            [(Vec3::Z * 8.0, 2, 1, 5.5), (Vec3::ZERO, 1, 1, 5.5)].into_iter(),
        );
        assert_eq!(pick.map(|pick| pick.node_id), Some(1));
    }
}
