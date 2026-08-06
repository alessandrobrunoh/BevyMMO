//! Systems for the boss bar and phase banner.
//!
//! The bar is built once in `Startup` and kept hidden; the update system toggles
//! its visibility and fills it from the boss's replicated `VitalStats`. Phase
//! transitions are detected locally (last vs current `BossPhase`) and drive a
//! transient banner — no extra network message is needed.

use bevy::prelude::*;

use bevymmo_shared::entity::boss::components::{Boss, BossArena, BossPhase};
use bevymmo_shared::stats::components::VitalStats;

use crate::game_state::{GameScreen, Screen};
use crate::ui::theme::UiTheme;

use super::components::{BossBanner, BossBannerState, BossBarFill, BossBarRoot};

const BAR_WIDTH_PERCENT: f32 = 40.0;
const BAR_HEIGHT_PX: f32 = 18.0;
const BANNER_SECONDS: f32 = 2.0;

/// Builds the bar and banner nodes (hidden by default) in `Startup`.
pub fn setup_boss_bar(mut commands: Commands, theme: Res<UiTheme>) {
    // Bar root, anchored top-center.
    let bar_root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(24.0),
                left: Val::Percent(50.0),
                margin: UiRect::new(
                    Val::Percent(0.0),
                    Val::Percent(0.0),
                    Val::Px(0.0),
                    Val::Px(0.0),
                ),
                width: Val::Percent(BAR_WIDTH_PERCENT),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                display: Display::None,
                ..default()
            },
            BossBarRoot,
        ))
        .id();

    let name_text = commands
        .spawn((
            Text::new("Vermithrax, the Ashen Drake".to_string()),
            TextFont {
                font_size: FontSize::Px(theme.scoreboard_entry_size),
                ..default()
            },
            TextColor(theme.text_color),
        ))
        .id();
    commands.entity(bar_root).add_child(name_text);

    // Bar background (dark) containing the fill.
    let bar_bg = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(BAR_HEIGHT_PX),
                ..default()
            },
            BackgroundColor(theme.bar_bg),
        ))
        .id();
    commands.entity(bar_root).add_child(bar_bg);

    let fill = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(theme.hp_fill),
            BossBarFill,
        ))
        .id();
    commands.entity(bar_bg).add_child(fill);

    // Banner, anchored screen center.
    let _banner = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(40.0),
                left: Val::Percent(50.0),
                margin: UiRect::new(
                    Val::Percent(0.0),
                    Val::Percent(0.0),
                    Val::Px(0.0),
                    Val::Px(0.0),
                ),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            BossBanner,
        ))
        .id();
}

/// Toggles bar visibility, updates the fill width/color, and detects phase
/// transitions to arm the banner.
///
/// `roots` and `fills` both take `&mut Node`; Bevy's B0001 check can't prove
/// they're disjoint (root vs. its fill child), so they're wrapped in a
/// `ParamSet` and accessed sequentially. The banner query is separate because
/// `BossBanner` is provably disjoint from both markers.
pub fn update_boss_bar(
    mut commands: Commands,
    theme: Res<UiTheme>,
    mut banner_state: ResMut<BossBannerState>,
    bosses: Query<(&VitalStats, &BossArena, &BossPhase), With<Boss>>,
    mut bar_params: ParamSet<(
        Query<&mut Node, With<BossBarRoot>>,
        Query<(&mut Node, &mut BackgroundColor), With<BossBarFill>>,
    )>,
    banners: Query<(Entity, Option<&Children>), With<BossBanner>>,
) {
    // Resolve the (single) boss, if any.
    let boss = bosses.iter().next();

    let should_show_bar = boss.is_some_and(|(vital, arena, phase)| {
        arena.is_engaged && !vital.is_dead() && *phase != BossPhase::Dead
    });

    {
        let mut roots = bar_params.p0();
        for mut root in roots.iter_mut() {
            root.display = if should_show_bar {
                Display::Flex
            } else {
                Display::None
            };
        }
    }

    if should_show_bar {
        let (vital, _arena, phase) = boss.expect("checked above");
        let fraction = if vital.max_health > 0.0 {
            (vital.current_health / vital.max_health).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let fill_color = fill_color_for_phase(*phase, &theme);
        let mut fills = bar_params.p1();
        for (mut node, mut bg) in fills.iter_mut() {
            node.width = Val::Percent(fraction * 100.0);
            bg.0 = fill_color;
        }
    }

    // Phase-change detection drives the banner.
    let current_phase = boss.map(|(_, _, phase)| *phase);
    if current_phase != banner_state.last_phase {
        if let Some(from) = banner_state.last_phase {
            let to = current_phase.unwrap_or(BossPhase::Dormant);
            if let Some(text) = banner_text_for_transition(from, to) {
                show_banner(&mut commands, &banners, text, &theme);
                banner_state.remaining_seconds = BANNER_SECONDS;
            }
        }
        banner_state.last_phase = current_phase;
    }
}

/// Shows the banner node (via `Commands`, to avoid a second `&mut Node` query
/// that would conflict with the bar queries) and writes the banner text.
fn show_banner(
    commands: &mut Commands,
    banners: &Query<(Entity, Option<&Children>), With<BossBanner>>,
    text: &str,
    theme: &UiTheme,
) {
    let Ok((banner_entity, children)) = banners.single() else {
        return;
    };
    commands.entity(banner_entity).insert(Node {
        display: Display::Flex,
        ..default()
    });

    // Replace the text child if present, else spawn one.
    let existing_text = children
        .and_then(|kids| kids.iter().next())
        .filter(|child| commands.get_entity(*child).is_ok());

    if let Some(text_entity) = existing_text {
        commands
            .entity(text_entity)
            .insert((Text::new(text.to_string()),));
    } else {
        let text_id = commands
            .spawn((
                Text::new(text.to_string()),
                TextFont {
                    font_size: FontSize::Px(theme.title_font_size),
                    ..default()
                },
                TextColor(theme.text_color),
            ))
            .id();
        commands.entity(banner_entity).add_child(text_id);
    }
}

/// Hides the banner when the timer reaches zero.
pub fn update_boss_banner(
    banner_state: Res<BossBannerState>,
    mut banners: Query<&mut Node, With<BossBanner>>,
) {
    let visible = banner_state.remaining_seconds > 0.0;
    for mut node in banners.iter_mut() {
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
}

/// Decrements the banner timer each frame (only while in gameplay).
pub fn tick_boss_banner(
    time: Res<Time>,
    screen: Res<GameScreen>,
    mut banner_state: ResMut<BossBannerState>,
) {
    if !matches!(screen.0, Screen::InGame | Screen::Paused) {
        return;
    }
    banner_state.remaining_seconds = (banner_state.remaining_seconds - time.delta_secs()).max(0.0);
}

/// Fill color shifts per phase so the bar communicates progress at a glance.
fn fill_color_for_phase(phase: BossPhase, theme: &UiTheme) -> Color {
    match phase {
        BossPhase::Ground => Color::srgb(0.55, 0.05, 0.05),
        BossPhase::Aerial => Color::srgb(1.0, 0.45, 0.05),
        BossPhase::Berserk => Color::srgb(1.0, 0.80, 0.15),
        _ => theme.hp_fill,
    }
}

/// Pure mapping from a phase transition to its banner copy.
///
/// `None` means "no banner" (e.g. initial spawn or transitions out of Dormant
/// other than engage). Exposed for unit testing.
pub fn banner_text_for_transition(from: BossPhase, to: BossPhase) -> Option<&'static str> {
    match (from, to) {
        (BossPhase::Dormant, BossPhase::Ground) => Some("VERMITHRAX AWAKENS"),
        (_, BossPhase::Aerial) => Some("PHASE 2 — TAKE FLIGHT"),
        (_, BossPhase::Berserk) => Some("BERSERK!"),
        (_, BossPhase::Dead) => Some("VERMITHRAX FALLS"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engage_banner_fires_on_dormant_to_ground() {
        assert_eq!(
            banner_text_for_transition(BossPhase::Dormant, BossPhase::Ground),
            Some("VERMITHRAX AWAKENS")
        );
    }

    #[test]
    fn aerial_banner_fires_from_any_prior() {
        assert_eq!(
            banner_text_for_transition(BossPhase::Ground, BossPhase::Aerial),
            Some("PHASE 2 — TAKE FLIGHT")
        );
    }

    #[test]
    fn berserk_banner_fires_on_any_entry() {
        assert_eq!(
            banner_text_for_transition(BossPhase::Ground, BossPhase::Berserk),
            Some("BERSERK!")
        );
        assert_eq!(
            banner_text_for_transition(BossPhase::Aerial, BossPhase::Berserk),
            Some("BERSERK!")
        );
    }

    #[test]
    fn death_banner_fires_on_dead() {
        assert_eq!(
            banner_text_for_transition(BossPhase::Berserk, BossPhase::Dead),
            Some("VERMITHRAX FALLS")
        );
    }

    #[test]
    fn no_banner_for_unchanged_or_initial_phase() {
        assert_eq!(
            banner_text_for_transition(BossPhase::Ground, BossPhase::Ground),
            None
        );
        assert_eq!(
            banner_text_for_transition(BossPhase::Dormant, BossPhase::Dormant),
            None
        );
    }
}
