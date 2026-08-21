//! Left-click a harvestable node to start a server-authoritative gather.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::app_state::in_unpaused_gameplay;
use crate::local_player::LocalPlayer;
use crate::movement::cursor_ray;
use crate::pointer::{hud_wants_pointer, PointerOnHud};
use crate::server_feed::ServerNotice;
use crate::stdb::{commands, StdbConnection};
use bevymmo_gameplay::entity::components::GameEntity;
use bevymmo_gameplay::gathering::Harvestable;
use bevymmo_network::world_components::{NetworkEntityId, Position};

const GATHER_SELECT_RADIUS: f32 = 2.0;
const DEPLETED_MESSAGE: &str = "Questa risorsa è già stata completamente raccolta";

/// World click → `start_gather`.
pub struct GatheringPlugin;

impl Plugin for GatheringPlugin {
    fn build(&self, app: &mut App) {
        crate::pointer::PointerPlugin::ensure(app);
        app.add_systems(Update, click_resource_node.run_if(in_unpaused_gameplay));
    }
}

fn click_resource_node(
    mouse: Res<ButtonInput<MouseButton>>,
    pointer_on_hud: Res<PointerOnHud>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &Transform), With<Camera3d>>,
    conn: Option<Res<StdbConnection>>,
    nodes: Query<
        (&Position, &NetworkEntityId, &Harvestable),
        (With<GameEntity>, Without<LocalPlayer>),
    >,
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

    let mut closest: Option<(u64, f32, u32)> = None;
    for (position, network_id, harvestable) in nodes.iter() {
        let oc = position.0 - ray.origin;
        let t = oc.dot(*ray.direction);
        if t < 0.0 {
            continue;
        }
        let closest_point = ray.origin + *ray.direction * t;
        let distance = closest_point.distance(position.0);
        if distance > GATHER_SELECT_RADIUS {
            continue;
        }
        if closest.is_none_or(|(_, best, _)| t < best) {
            closest = Some((network_id.0, t, harvestable.current_pieces));
        }
    }

    let Some((node_id, _, pieces)) = closest else {
        return;
    };
    if pieces == 0 {
        notices.write(ServerNotice::error(DEPLETED_MESSAGE));
        return;
    }
    if let Err(err) = commands::start_gather(&conn, node_id) {
        error!("start_gather failed: {err}");
    }
}
