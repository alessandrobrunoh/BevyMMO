use bevy::prelude::*;
use bevymmo_shared::entity::LocalPlayer;

use bevymmo_client::network::types::ClientConnectionConfig;
use bevymmo_shared::movement::effective_movement_speed;
use bevymmo_shared::network::protocol::{NetworkEntityId, PlayerId};
use bevymmo_shared::stats::components::{CombatStats, MovementStats, VitalStats};
use bevymmo_shared::stats::modifiers::ActiveStatModifiers;

use crate::game_state::{GameScreen, Screen};
use crate::spells::cast_bar::ObservedCasts;
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
    observed_casts: Option<Res<ObservedCasts>>,
    player_query: Query<(
        &MovementStats,
        &CombatStats,
        &VitalStats,
        Option<&PlayerId>,
        Has<LocalPlayer>,
        Option<&ActiveStatModifiers>,
        Option<&NetworkEntityId>,
    )>,
    mut root_query: Query<&mut Node, With<PlayerStatsUi>>,
    mut text_query: Query<&mut Text, With<PlayerStatsText>>,
    mut last_text: Local<String>,
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
    let Some((movement, combat, vital, _, _, modifiers, network_id)) = player_query
        .iter()
        .find(|(_, _, _, _, controlled, _, _)| *controlled)
        .or_else(|| {
            player_query.iter().find(|(_, _, _, player_id, _, _, _)| {
                player_id.is_some_and(|id| {
                    local_client_id.is_some_and(|client_id| id.0.to_bits() == client_id)
                })
            })
        })
    else {
        return;
    };
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let new_text = format_stats(
        movement,
        combat,
        vital,
        modifiers,
        network_id,
        observed_casts.as_deref(),
    );
    if *last_text != new_text {
        text.0 = new_text.clone();
        *last_text = new_text;
    }
}

fn format_stats(
    movement: &MovementStats,
    combat: &CombatStats,
    vital: &VitalStats,
    modifiers: Option<&ActiveStatModifiers>,
    network_id: Option<&NetworkEntityId>,
    observed_casts: Option<&ObservedCasts>,
) -> String {
    let move_speed =
        displayed_movement_speed(movement.speed, modifiers, network_id, observed_casts);
    format!(
        "HP: {}/{}\nMax Mana: {}\nMana Regen: {:.1}/s\nArmor: {} ({}% reduction)\nAttack Power: {}\nMove Speed: {:.2}",
        format_value(vital.current_health),
        format_value(vital.max_health),
        format_value(vital.max_mana),
        vital.mana_regeneration,
        format_value(combat.armor),
        combat.armor_damage_reduction() * 100.0,
        format_value(combat.attack_power),
        move_speed,
    )
}

fn displayed_movement_speed(
    base_speed: f32,
    modifiers: Option<&ActiveStatModifiers>,
    network_id: Option<&NetworkEntityId>,
    observed_casts: Option<&ObservedCasts>,
) -> f32 {
    let effective_speed = effective_movement_speed(base_speed, modifiers);
    if modifiers.is_some() {
        return effective_speed;
    }

    let (Some(network_id), Some(observed_casts)) = (network_id, observed_casts) else {
        return effective_speed;
    };
    if observed_casts
        .0
        .get(&network_id.0)
        .is_some_and(|cast| cast.spell_id == "swift")
    {
        return effective_speed * 1.35;
    }

    effective_speed
}

fn format_value(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}
