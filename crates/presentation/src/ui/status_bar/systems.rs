//! Rendering of semantic ActiveStatus snapshots.

use bevy::prelude::*;
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_gameplay::effects::{
    ActiveStatusSnapshot, ActiveStatuses, StatusCategory, StatusId, StatusRegistry,
};

use crate::game_state::{in_gameplay, Screen};
use crate::ui::target_frame::components::{TargetFrame, TargetFrameTarget};

/// Sits just above the ability hotbar (`bottom: 86`) so personal buffs are in
/// the same gaze as the ability hotbar instead of a thin strip at the top of the screen.
const STATUS_BAR_BOTTOM: f32 = 200.0;
const STATUS_CARD_WIDTH: f32 = 108.0;
const STATUS_CARD_HEIGHT: f32 = 72.0;
const STATUS_DURATION_BAR_WIDTH: f32 = 92.0;
const STATUS_DURATION_BAR_HEIGHT: f32 = 5.0;

#[derive(Component)]
struct StatusBarRoot;

#[derive(Component)]
struct StatusCard;

#[derive(Component, Default)]
struct StatusBarIdentity(Vec<(u64, u16)>);

#[derive(Component)]
struct StatusCardTimer {
    remaining: f32,
    total: f32,
}

#[derive(Component)]
struct StatusRemainingText;

#[derive(Component)]
struct StatusDurationFill;

pub struct StatusBarPlugin;

impl Plugin for StatusBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_status_bar).add_systems(
            Update,
            (
                sync_status_bar,
                sync_target_status_bar,
                tick_status_card_timers,
            )
                .chain(),
        );
    }
}

fn setup_status_bar(mut commands: Commands) {
    commands.spawn((
        StatusBarRoot,
        StatusBarIdentity::default(),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(STATUS_BAR_BOTTOM),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            height: Val::Px(STATUS_CARD_HEIGHT + 12.0),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        },
        GlobalZIndex(40),
        Visibility::Hidden,
        Pickable::IGNORE,
    ));
}

fn sync_status_bar(
    mut commands: Commands,
    local_player: Query<&ActiveStatuses, With<LocalPlayer>>,
    status_registry: Res<StatusRegistry>,
    screen: Res<State<Screen>>,
    root: Single<(Entity, &mut Visibility, &mut StatusBarIdentity), With<StatusBarRoot>>,
) {
    let (root_entity, mut visibility, mut identity) = root.into_inner();
    let playing = in_gameplay(screen);
    let empty = ActiveStatuses::default();
    let statuses = local_player.single().unwrap_or(&empty);
    if !playing || statuses.statuses.is_empty() {
        if !identity.0.is_empty() {
            commands.entity(root_entity).despawn_related::<Children>();
            identity.0.clear();
        }
        *visibility = Visibility::Hidden;
        return;
    }

    let next_identity = status_set_identity(statuses);
    if identity.0 == next_identity {
        *visibility = Visibility::Visible;
        return;
    }
    identity.0 = next_identity;

    commands.entity(root_entity).despawn_related::<Children>();
    *visibility = Visibility::Visible;
    let mut ordered = statuses.statuses.clone();
    ordered.sort_by_key(status_priority);

    commands.entity(root_entity).with_children(|parent| {
        for status in &ordered {
            spawn_status_card(parent, status, &status_registry, StatusCardSize::Full);
        }
    });
}

/// Marker for the status row parented under the selected-target frame.
#[derive(Component)]
pub(crate) struct TargetStatusRow;

/// Compact row under the target frame. Hidden until the target has statuses.
pub(crate) fn spawn_target_status_row(commands: &mut Commands, parent: Entity) {
    let row = commands
        .spawn((
            TargetStatusRow,
            StatusBarIdentity::default(),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(4.0),
                row_gap: Val::Px(4.0),
                ..default()
            },
            Visibility::Hidden,
        ))
        .id();
    commands.entity(parent).add_child(row);
}

fn sync_target_status_bar(
    mut commands: Commands,
    frames: Query<&TargetFrameTarget, With<TargetFrame>>,
    mut rows: Query<(Entity, &mut Visibility, &mut StatusBarIdentity), With<TargetStatusRow>>,
    targets: Query<&ActiveStatuses>,
    status_registry: Res<StatusRegistry>,
) {
    let Ok((row_entity, mut visibility, mut identity)) = rows.single_mut() else {
        return;
    };
    let Ok(frame) = frames.single() else {
        return;
    };
    let empty = ActiveStatuses::default();
    let statuses = targets.get(frame.entity).unwrap_or(&empty);
    let next_identity = status_set_identity(statuses);
    if identity.0 == next_identity {
        return;
    }
    identity.0 = next_identity;

    commands.entity(row_entity).despawn_related::<Children>();
    if statuses.statuses.is_empty() {
        *visibility = Visibility::Hidden;
        return;
    }

    *visibility = Visibility::Visible;
    let mut ordered = statuses.statuses.clone();
    ordered.sort_by_key(status_priority);
    commands.entity(row_entity).with_children(|parent| {
        for status in &ordered {
            spawn_status_card(parent, status, &status_registry, StatusCardSize::Compact);
        }
    });
}

fn status_set_identity(statuses: &ActiveStatuses) -> Vec<(u64, u16)> {
    let mut identity: Vec<_> = statuses
        .statuses
        .iter()
        .map(|status| (status.instance_id, status.stacks))
        .collect();
    identity.sort_unstable();
    identity
}

fn tick_status_card_timers(
    time: Res<Time>,
    mut cards: Query<(&mut StatusCardTimer, &Children)>,
    mut remaining_text: Query<&mut Text, With<StatusRemainingText>>,
    mut fills: Query<&mut Node, With<StatusDurationFill>>,
    children_query: Query<&Children>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (mut timer, children) in &mut cards {
        timer.remaining = (timer.remaining - dt).max(0.0);
        let remaining = format_remaining(timer.remaining);
        let ratio = duration_ratio(timer.remaining, timer.total);
        visit_descendants(children, &children_query, &mut |entity| {
            if let Ok(mut label) = remaining_text.get_mut(entity) {
                label.0 = remaining.clone();
            }
            if let Ok(mut node) = fills.get_mut(entity) {
                node.width = Val::Percent(ratio * 100.0);
            }
        });
    }
}

/// Iterative walk: a recursive `impl FnMut` re-monomorphizes on every call
/// (`&mut visit`) and blows the rustc recursion limit.
fn visit_descendants(children: &Children, tree: &Query<&Children>, visit: &mut dyn FnMut(Entity)) {
    let mut stack: Vec<Entity> = children.iter().collect();
    while let Some(entity) = stack.pop() {
        visit(entity);
        if let Ok(nested) = tree.get(entity) {
            stack.extend(nested.iter());
        }
    }
}

fn status_priority(status: &ActiveStatusSnapshot) -> (u8, u64) {
    // Hard control is kept closest to the center; the current snapshot does not
    // carry the static definition, so the stable fallback is the instance id.
    (0, status.instance_id)
}

#[derive(Clone, Copy)]
enum StatusCardSize {
    Full,
    Compact,
}

impl StatusCardSize {
    fn width(self) -> f32 {
        match self {
            Self::Full => STATUS_CARD_WIDTH,
            Self::Compact => 62.0,
        }
    }

    fn height(self) -> f32 {
        match self {
            Self::Full => STATUS_CARD_HEIGHT,
            Self::Compact => 48.0,
        }
    }

    fn name_font(self) -> f32 {
        match self {
            Self::Full => 14.0,
            Self::Compact => 10.0,
        }
    }

    fn show_category_label(self) -> bool {
        matches!(self, Self::Full)
    }
}

fn spawn_status_card(
    parent: &mut ChildSpawnerCommands,
    status: &ActiveStatusSnapshot,
    registry: &StatusRegistry,
    size: StatusCardSize,
) {
    let definition = registry.get(&StatusId::new(status.status_id.clone()));
    let category_kind = definition
        .as_ref()
        .map(|definition| definition.category)
        .unwrap_or(StatusCategory::Debuff);
    let category = category_color(category_kind);
    let display_name = definition
        .as_ref()
        .map(|definition| definition.presentation.short_name)
        .unwrap_or(status.status_id.as_str());
    let icon = definition
        .as_ref()
        .map(|definition| status_icon_text(definition.presentation.icon))
        .unwrap_or("◆");
    let stack_suffix = if status.stacks > 1 {
        format!(" ×{}", status.stacks)
    } else {
        String::new()
    };
    let remaining = format_remaining(status.remaining_seconds);
    let duration_ratio = duration_ratio(status.remaining_seconds, status.total_seconds);
    let category_name = match category_kind {
        StatusCategory::Buff => "BUFF",
        StatusCategory::Debuff => "DEBUFF",
    };

    parent
        .spawn((
            StatusCard,
            StatusCardTimer {
                remaining: status.remaining_seconds,
                total: status.total_seconds,
            },
            Node {
                width: Val::Px(size.width()),
                height: Val::Px(size.height()),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(5.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(2.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.05, 0.08, 0.92)),
            BorderColor::all(category),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new(format!("{} {}{}", icon, display_name, stack_suffix)),
                TextFont {
                    font_size: FontSize::Px(size.name_font()),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            if size.show_category_label() {
                card.spawn((
                    Text::new(category_name),
                    TextFont {
                        font_size: FontSize::Px(8.0),
                        ..default()
                    },
                    TextColor(category),
                ));
            }
            card.spawn((
                StatusRemainingText,
                Text::new(remaining),
                TextFont {
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(Color::srgba(0.85, 0.88, 0.95, 1.0)),
            ));
            card.spawn((
                Node {
                    width: Val::Px(if matches!(size, StatusCardSize::Compact) {
                        50.0
                    } else {
                        STATUS_DURATION_BAR_WIDTH
                    }),
                    height: Val::Px(STATUS_DURATION_BAR_HEIGHT),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.06, 0.09, 0.8)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    StatusDurationFill,
                    Node {
                        width: Val::Percent(duration_ratio * 100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(category),
                ));
            });
        });
}

fn status_icon_text(icon_id: &str) -> &str {
    match icon_id {
        "status_burn" => "🔥",
        "status_stun" => "✦",
        "status_swift" => "⚡",
        "status_slow" => "❄",
        "status_root" => "⚓",
        _ => "◆",
    }
}

fn duration_ratio(remaining_seconds: f32, total_seconds: f32) -> f32 {
    if !remaining_seconds.is_finite() || !total_seconds.is_finite() || total_seconds <= 0.0 {
        return 0.0;
    }

    (remaining_seconds / total_seconds).clamp(0.0, 1.0)
}

fn category_color(category: StatusCategory) -> Color {
    match category {
        StatusCategory::Buff => Color::srgb(0.18, 0.55, 0.95),
        StatusCategory::Debuff => Color::srgb(0.85, 0.25, 0.25),
    }
}

fn format_remaining(seconds: f32) -> String {
    if seconds >= 10.0 {
        format!("{seconds:.0}s")
    } else {
        format!("{seconds:.1}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::target_frame::systems::manage_target_frame;
    use crate::ui::theme::UiTheme;
    use bevymmo_client::targeting::CurrentTarget;
    use bevymmo_gameplay::entity::components::{EntityKind, PlayerName};
    use bevymmo_gameplay::stats::components::VitalStats;
    use bevymmo_network::network::protocol::Position;

    #[test]
    fn remaining_time_uses_compact_display_precision() {
        assert_eq!(format_remaining(12.4), "12s");
        assert_eq!(format_remaining(2.36), "2.4s");
    }

    #[test]
    fn status_cards_have_stable_priority_by_instance_id() {
        let first = ActiveStatusSnapshot {
            instance_id: 1,
            status_id: "burn".to_string(),
            source: None,
            stacks: 1,
            potency: 1.0,
            remaining_seconds: 1.0,
            total_seconds: 5.0,
        };
        let second = ActiveStatusSnapshot {
            instance_id: 2,
            ..first.clone()
        };

        assert!(status_priority(&first) < status_priority(&second));
    }

    #[test]
    fn status_category_color_has_a_buff_override() {
        assert_ne!(
            category_color(StatusCategory::Buff),
            category_color(StatusCategory::Debuff)
        );
    }

    #[test]
    fn status_icons_have_a_neutral_fallback() {
        assert_eq!(status_icon_text("status_burn"), "🔥");
        assert_eq!(status_icon_text("unknown"), "◆");
    }

    #[test]
    fn status_set_identity_ignores_remaining_seconds() {
        let mut statuses = ActiveStatuses::default();
        statuses.statuses.push(ActiveStatusSnapshot {
            instance_id: 3,
            status_id: "slow".to_string(),
            source: None,
            stacks: 1,
            potency: 1.0,
            remaining_seconds: 3.0,
            total_seconds: 3.0,
        });
        let first = status_set_identity(&statuses);
        statuses.statuses[0].remaining_seconds = 0.4;
        assert_eq!(first, status_set_identity(&statuses));
        statuses.statuses[0].stacks = 2;
        assert_ne!(first, status_set_identity(&statuses));
    }

    #[test]
    fn duration_ratio_is_clamped_and_safe_for_invalid_totals() {
        assert_eq!(duration_ratio(5.0, 10.0), 0.5);
        assert_eq!(duration_ratio(20.0, 10.0), 1.0);
        assert_eq!(duration_ratio(-1.0, 10.0), 0.0);
        assert_eq!(duration_ratio(1.0, 0.0), 0.0);
    }

    fn snapshot(status_id: &str, instance_id: u64) -> ActiveStatusSnapshot {
        ActiveStatusSnapshot {
            instance_id,
            status_id: status_id.to_string(),
            source: None,
            stacks: 1,
            potency: 1.0,
            remaining_seconds: 4.0,
            total_seconds: 5.0,
        }
    }

    fn target_status_app(statuses: Vec<ActiveStatusSnapshot>) -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(bevymmo_content::status_definitions::default_statuses());
        app.init_resource::<UiTheme>();
        app.add_systems(
            Update,
            (manage_target_frame, sync_target_status_bar).chain(),
        );

        let target = app
            .world_mut()
            .spawn((
                Position(Vec3::ZERO),
                VitalStats {
                    current_health: 80.0,
                    max_health: 100.0,
                    current_mana: 0.0,
                    max_mana: 0.0,
                    mana_regeneration: 0.0,
                },
                PlayerName("Dummy".to_string()),
                EntityKind::Neutral,
                ActiveStatuses { statuses },
            ))
            .id();
        app.world_mut().insert_resource(CurrentTarget::new(target));
        (app, target)
    }

    #[test]
    fn target_status_row_stays_hidden_when_the_target_has_no_statuses() {
        let (mut app, _) = target_status_app(Vec::new());
        app.update();

        let mut vis = app
            .world_mut()
            .query_filtered::<&Visibility, With<TargetStatusRow>>();
        let visibility = vis.single(app.world()).expect("status row");
        assert!(matches!(*visibility, Visibility::Hidden));
    }

    #[test]
    fn target_status_row_shows_a_card_for_each_status() {
        let (mut app, _) = target_status_app(vec![snapshot("slow", 1), snapshot("burn", 2)]);
        app.update();

        let mut vis = app
            .world_mut()
            .query_filtered::<&Visibility, With<TargetStatusRow>>();
        assert!(matches!(
            *vis.single(app.world()).expect("status row"),
            Visibility::Visible
        ));

        let cards = app
            .world_mut()
            .query::<&StatusCard>()
            .iter(app.world())
            .count();
        assert_eq!(cards, 2);
    }

    fn player_status_app(statuses: Vec<ActiveStatusSnapshot>) -> App {
        let mut app = App::new();
        app.insert_resource(bevymmo_content::status_definitions::default_statuses());
        crate::game_state::init_screen_states(&mut app);
        app.insert_state(Screen::InGame);
        app.add_systems(Startup, setup_status_bar);
        app.add_systems(Update, sync_status_bar);
        app.world_mut()
            .spawn((LocalPlayer, ActiveStatuses { statuses }));
        app
    }

    #[test]
    fn player_status_bar_stays_hidden_without_statuses() {
        let mut app = player_status_app(Vec::new());
        app.update();
        let mut vis = app
            .world_mut()
            .query_filtered::<&Visibility, With<StatusBarRoot>>();
        assert!(matches!(
            *vis.single(app.world()).expect("player status bar"),
            Visibility::Hidden
        ));
    }

    #[test]
    fn player_status_bar_shows_a_card_for_each_local_status() {
        let mut app = player_status_app(vec![snapshot("swift", 1), snapshot("slow", 2)]);
        app.update();

        let mut vis = app
            .world_mut()
            .query_filtered::<&Visibility, With<StatusBarRoot>>();
        assert!(matches!(
            *vis.single(app.world()).expect("player status bar"),
            Visibility::Visible
        ));
        let cards = app
            .world_mut()
            .query::<&StatusCard>()
            .iter(app.world())
            .count();
        assert_eq!(cards, 2);
    }

    #[test]
    fn player_status_bar_appears_when_local_player_is_tagged_after_statuses() {
        let mut app = App::new();
        app.insert_resource(bevymmo_content::status_definitions::default_statuses());
        crate::game_state::init_screen_states(&mut app);
        app.insert_state(Screen::InGame);
        app.add_systems(Startup, setup_status_bar);
        app.add_systems(Update, sync_status_bar);

        let entity = app
            .world_mut()
            .spawn(ActiveStatuses {
                statuses: vec![snapshot("burn", 9)],
            })
            .id();
        app.update();
        app.world_mut().entity_mut(entity).insert(LocalPlayer);
        app.update();

        let mut vis = app
            .world_mut()
            .query_filtered::<&Visibility, With<StatusBarRoot>>();
        assert!(matches!(
            *vis.single(app.world()).expect("player status bar"),
            Visibility::Visible
        ));
        assert_eq!(
            app.world_mut()
                .query::<&StatusCard>()
                .iter(app.world())
                .count(),
            1
        );
    }

    #[test]
    fn player_status_bar_hides_outside_gameplay() {
        let mut app = player_status_app(vec![snapshot("swift", 1)]);
        app.update();
        app.insert_state(Screen::MainMenu);
        app.update();

        let mut vis = app
            .world_mut()
            .query_filtered::<&Visibility, With<StatusBarRoot>>();
        assert!(matches!(
            *vis.single(app.world()).expect("player status bar"),
            Visibility::Hidden
        ));
    }
}
