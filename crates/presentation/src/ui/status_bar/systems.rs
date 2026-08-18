//! Rendering of semantic ActiveStatus snapshots.

use bevy::prelude::*;
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_gameplay::effects::{
    ActiveStatusSnapshot, ActiveStatuses, StatusCategory, StatusId, StatusRegistry,
};

const STATUS_BAR_TOP: f32 = 18.0;
const STATUS_BAR_WIDTH_PERCENT: f32 = 90.0;
const STATUS_CARD_WIDTH: f32 = 92.0;
const STATUS_CARD_HEIGHT: f32 = 62.0;
const STATUS_DURATION_BAR_WIDTH: f32 = 78.0;
const STATUS_DURATION_BAR_HEIGHT: f32 = 3.0;

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
        app.add_systems(Startup, setup_status_bar)
            .add_systems(Update, (sync_status_bar, tick_status_card_timers).chain());
    }
}

fn setup_status_bar(mut commands: Commands) {
    commands.spawn((
        StatusBarRoot,
        StatusBarIdentity::default(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(STATUS_BAR_TOP),
            left: Val::Percent(50.0),
            width: Val::Percent(STATUS_BAR_WIDTH_PERCENT),
            margin: UiRect::left(Val::Percent(-STATUS_BAR_WIDTH_PERCENT / 2.0)),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Start,
            column_gap: Val::Px(6.0),
            ..default()
        },
        Visibility::Hidden,
    ));
}

fn sync_status_bar(
    mut commands: Commands,
    local_player: Query<&ActiveStatuses, (With<LocalPlayer>, Changed<ActiveStatuses>)>,
    status_registry: Res<StatusRegistry>,
    root: Single<(Entity, &mut Visibility, &mut StatusBarIdentity), With<StatusBarRoot>>,
) {
    let Ok(statuses) = local_player.single() else {
        return;
    };
    let (root_entity, mut visibility, mut identity) = root.into_inner();
    let next_identity = status_set_identity(statuses);
    if identity.0 == next_identity {
        return;
    }
    identity.0 = next_identity;

    commands.entity(root_entity).despawn_related::<Children>();
    if statuses.statuses.is_empty() {
        *visibility = Visibility::Hidden;
        return;
    }

    *visibility = Visibility::Visible;
    let mut ordered = statuses.statuses.clone();
    ordered.sort_by_key(status_priority);

    commands.entity(root_entity).with_children(|parent| {
        for status in &ordered {
            spawn_status_card(parent, status, &status_registry);
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
        visit_descendants(children, &children_query, |entity| {
            if let Ok(mut label) = remaining_text.get_mut(entity) {
                label.0 = remaining.clone();
            }
            if let Ok(mut node) = fills.get_mut(entity) {
                node.width = Val::Percent(ratio * 100.0);
            }
        });
    }
}

fn visit_descendants(children: &Children, tree: &Query<&Children>, mut visit: impl FnMut(Entity)) {
    for child in children {
        visit(*child);
        if let Ok(nested) = tree.get(*child) {
            visit_descendants(nested, tree, &mut visit);
        }
    }
}

fn status_priority(status: &ActiveStatusSnapshot) -> (u8, u64) {
    // Hard control is kept closest to the center; the current snapshot does not
    // carry the static definition, so the stable fallback is the instance id.
    (0, status.instance_id)
}

fn spawn_status_card(
    parent: &mut ChildSpawnerCommands,
    status: &ActiveStatusSnapshot,
    registry: &StatusRegistry,
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
                width: Val::Px(STATUS_CARD_WIDTH),
                height: Val::Px(STATUS_CARD_HEIGHT),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(category.with_alpha(0.22)),
            BorderColor::all(category),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new(format!("{} {}{}", icon, display_name, stack_suffix)),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            card.spawn((
                Text::new(category_name),
                TextFont {
                    font_size: FontSize::Px(8.0),
                    ..default()
                },
                TextColor(category),
            ));
            card.spawn((
                StatusRemainingText,
                Text::new(remaining),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(Color::srgba(0.85, 0.88, 0.95, 1.0)),
            ));
            card.spawn((
                Node {
                    width: Val::Px(STATUS_DURATION_BAR_WIDTH),
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
}
