//! Local gather progress bar and left-click to start gathering.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::game_state::Screen;
use crate::ui::npc_sidebar::systems::cursor_ray;
use crate::ui::theme::UiTheme;
use bevymmo_client::gathering::{
    gather_click_action, interact_range_for, nearest_harvestable_in_range, pick_harvestable,
    pick_height_for, GatherClick, GatherClickAction, TOO_FAR_MESSAGE,
};
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_client::pointer::{hud_wants_pointer, PointerOnHud};
use bevymmo_client::server_feed::ServerNotice;
use bevymmo_client::stdb::{commands, StdbConnection};
use bevymmo_gameplay::crafting::ActiveCraft;
use bevymmo_gameplay::gathering::{in_interact_range, ActiveGather, Harvestable};
use bevymmo_gameplay::items::registry::ItemRegistry;
use bevymmo_gameplay::placeables::PlaceableRegistry;
use bevymmo_network::world_components::{NetworkEntityId, Position};

pub struct GatherBarPlugin;

impl Plugin for GatherBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_gather_bar);
        app.add_systems(
            Update,
            (harvestable_on_click, update_gather_bar)
                .chain()
                .run_if(in_state(Screen::InGame)),
        );
    }
}

/// Same entry point as [`crate::ui::market::systems::npc_market_on_click`]:
/// left click, HUD check, camera ray, closest harvestable, then a reducer.
fn harvestable_on_click(
    mouse: Res<ButtonInput<MouseButton>>,
    pointer_on_hud: Res<PointerOnHud>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &Transform), With<Camera3d>>,
    conn: Option<Res<StdbConnection>>,
    placeables: Option<Res<PlaceableRegistry>>,
    player: Query<&Position, With<LocalPlayer>>,
    nodes: Query<(&Position, &NetworkEntityId, &Harvestable)>,
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

    let pick = pick_harvestable(
        ray,
        nodes.iter().map(|(position, network_id, harvestable)| {
            (
                position.0,
                network_id.0,
                harvestable.current_pieces,
                pick_height_for(harvestable.kind_id.as_str(), placeables.as_deref()),
            )
        }),
    );
    let already_gathering = local_gather.iter().next().is_some();
    let hit_node = pick.map(|pick| (pick.node_id, pick.pieces)).or_else(|| {
        let player_pos = player.single().ok()?.0;
        nearest_harvestable_in_range(
            player_pos,
            nodes.iter().map(|(position, network_id, harvestable)| {
                (
                    position.0,
                    network_id.0,
                    harvestable.current_pieces,
                    interact_range_for(harvestable.kind_id.as_str(), placeables.as_deref()),
                )
            }),
        )
    });

    match gather_click_action(GatherClick {
        hit_node,
        already_gathering,
    }) {
        GatherClickAction::Start(node_id) => {
            if let Some(pick) = pick {
                if let Ok(player_pos) = player.single() {
                    let kind_id = nodes.iter().find_map(|(_, network_id, harvestable)| {
                        (network_id.0 == node_id).then_some(harvestable.kind_id.as_str())
                    });
                    if let Some(kind_id) = kind_id {
                        let range = interact_range_for(kind_id, placeables.as_deref());
                        if !in_interact_range(
                            player_pos.0.x,
                            player_pos.0.z,
                            pick.position.x,
                            pick.position.z,
                            range,
                        ) {
                            notices.write(ServerNotice::error(TOO_FAR_MESSAGE));
                        }
                    }
                }
            }
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
            notices.write(ServerNotice::error(
                "Questa risorsa è già stata completamente raccolta",
            ));
        }
        GatherClickAction::None => {}
    }
}

#[derive(Component)]
struct GatherBarRoot;

#[derive(Component)]
struct GatherBarFill;

#[derive(Component)]
struct GatherBarLabel;

fn setup_gather_bar(mut commands: Commands, theme: Res<UiTheme>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(120.0),
                left: Val::Percent(50.0),
                width: Val::Px(240.0),
                height: Val::Px(22.0),
                margin: UiRect::left(Val::Px(-120.0)),
                border: UiRect::all(Val::Px(2.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.8)),
            BorderColor::all(theme.input_border),
            Pickable::IGNORE,
            GatherBarRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.85, 0.55, 0.15)),
                GatherBarFill,
            ));
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(theme.text_color),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                GatherBarLabel,
            ));
        });
}

fn update_gather_bar(
    gather: Query<&ActiveGather, With<LocalPlayer>>,
    craft: Query<&ActiveCraft, With<LocalPlayer>>,
    items: Option<Res<ItemRegistry>>,
    mut root: Query<&mut Node, With<GatherBarRoot>>,
    mut fill: Query<&mut Node, (With<GatherBarFill>, Without<GatherBarRoot>)>,
    mut label: Query<&mut Text, With<GatherBarLabel>>,
) {
    let gathering = gather.iter().next();
    let crafting = craft.iter().next();
    let Ok(mut root) = root.single_mut() else {
        return;
    };
    let (elapsed, required, caption) = if let Some(gather) = gathering {
        (
            gather.elapsed_seconds,
            gather.required_seconds,
            "Gathering".to_string(),
        )
    } else if let Some(craft) = crafting {
        let name = items
            .as_ref()
            .and_then(|registry| registry.get(&craft.item_id))
            .map(|item| item.display_name().to_string())
            .unwrap_or_else(|| craft.item_id.as_str().to_string());
        (
            craft.elapsed_seconds,
            craft.required_seconds,
            format!("Crafting {name}"),
        )
    } else {
        root.display = Display::None;
        return;
    };
    root.display = Display::Flex;
    let pct = if required <= 0.0 {
        1.0
    } else {
        (elapsed / required).clamp(0.0, 1.0)
    };
    if let Ok(mut fill) = fill.single_mut() {
        fill.width = Val::Percent(pct * 100.0);
    }
    if let Ok(mut text) = label.single_mut() {
        let remaining = (required - elapsed).max(0.0);
        text.0 = format!("{caption} {remaining:.1}s");
    }
}
