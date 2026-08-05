use bevy::prelude::*;
use lightyear::prelude::Controlled;

use crate::game_state::{GameScreen, Screen};
use crate::network::client::ClientConnectionConfig;
use crate::network::protocol::PlayerId;
use crate::plugins::entity::components::Stats;
use crate::ui::text::spawn_text;
use crate::ui::theme::UiTheme;

use super::plugin::{PlayerStatsText, PlayerStatsUi};

const PANEL_OFFSET: f32 = 16.0;

pub fn setup_player_stats(mut commands: Commands, theme: Res<UiTheme>) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(PANEL_OFFSET),
                right: Val::Px(PANEL_OFFSET),
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(theme.panel_bg),
            PlayerStatsUi,
        ))
        .id();

    let text = spawn_text(
        &mut commands,
        root,
        "Waiting for Player stats...",
        theme.hp_font_size,
        theme.text_color,
    );
    commands.entity(text).insert(PlayerStatsText);
}

pub fn update_player_stats(
    screen: Res<GameScreen>,
    client_config: Option<Res<ClientConnectionConfig>>,
    player_query: Query<(&Stats, Option<&PlayerId>, Has<Controlled>)>,
    mut root_query: Query<&mut Node, With<PlayerStatsUi>>,
    mut text_query: Query<&mut Text, With<PlayerStatsText>>,
) {
    let Ok(mut root) = root_query.single_mut() else {
        return;
    };

    if !matches!(screen.0, Screen::InGame | Screen::Paused) {
        root.display = Display::None;
        return;
    }

    root.display = Display::Flex;
    let local_client_id = client_config.map(|config| config.client_id);
    let Some(stats) = player_query
        .iter()
        .find(|(_, _, controlled)| *controlled)
        .or_else(|| {
            player_query.iter().find(|(_, player_id, _)| {
                player_id.is_some_and(|id| {
                    local_client_id.is_some_and(|client_id| id.0.to_bits() == client_id)
                })
            })
        })
        .map(|(stats, _, _)| stats)
    else {
        return;
    };
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    text.0 = format_stats(stats);
}

fn format_stats(stats: &Stats) -> String {
    format!(
        "Max HP: {}\nMax Mana: {}\nMana Regen: {:.1}/s\nArmor: {} ({}% reduction)",
        format_value(stats.max_health),
        format_value(stats.max_mana),
        stats.mana_regeneration,
        format_value(stats.armor),
        stats.damage_reduction() * 100.0,
    )
}

fn format_value(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}
