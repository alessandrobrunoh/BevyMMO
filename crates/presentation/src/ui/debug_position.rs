//! Debug overlay showing the local player's replicated position.

use bevy::prelude::*;
use bevymmo_client::local_player::LocalPlayer;

use bevymmo_client::network::types::ClientConnectionConfig;
use bevymmo_network::network::protocol::{PlayerId, Position};

use crate::game_state::{GameScreen, Screen};
use crate::ui::text::spawn_text;
use crate::ui::theme::UiTheme;

#[derive(Component)]
struct DebugPositionUi;

#[derive(Component)]
struct DebugPositionText;

pub struct DebugPositionPlugin;

impl Plugin for DebugPositionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_debug_position);
        app.add_systems(Update, update_debug_position);
    }
}

fn setup_debug_position(mut commands: Commands, theme: Res<UiTheme>) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                left: Val::Px(16.0),
                padding: UiRect::all(Val::Px(10.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(theme.panel_bg),
            DebugPositionUi,
        ))
        .id();

    let text = spawn_text(
        &mut commands,
        root,
        "Position\nX: --\nY: --\nZ: --",
        theme.hp_font_size,
        theme.text_color,
    );
    commands.entity(text).insert(DebugPositionText);
}

fn update_debug_position(
    screen: Res<GameScreen>,
    client_config: Option<Res<ClientConnectionConfig>>,
    players: Query<(&Position, Option<&PlayerId>, Has<LocalPlayer>)>,
    mut roots: Query<&mut Node, With<DebugPositionUi>>,
    mut texts: Query<&mut Text, With<DebugPositionText>>,
    mut last_text: Local<String>,
) {
    let Ok(mut root) = roots.single_mut() else {
        return;
    };

    if !matches!(screen.0, Screen::InGame | Screen::Paused) {
        root.display = Display::None;
        return;
    }

    let local_client_id = client_config.as_deref().map(|config| config.client_id);
    let local_position = players
        .iter()
        .find(|(_, _, controlled)| *controlled)
        .or_else(|| {
            players.iter().find(|(_, player_id, _)| {
                player_id.is_some_and(|id| {
                    local_client_id.is_some_and(|client_id| id.0.to_bits() == client_id)
                })
            })
        })
        .map(|(position, _, _)| position.0);

    let Some(position) = local_position else {
        root.display = Display::None;
        return;
    };

    root.display = Display::Flex;
    let new_text = format_position(position);
    if *last_text == new_text {
        return;
    }

    let Ok(mut text) = texts.single_mut() else {
        return;
    };
    text.0 = new_text.clone();
    *last_text = new_text;
}

fn format_position(position: Vec3) -> String {
    format!(
        "Position\nX: {:.2}\nY: {:.2}\nZ: {:.2}",
        position.x, position.y, position.z
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.init_resource::<UiTheme>();
        app.init_resource::<GameScreen>();
        app.add_plugins(DebugPositionPlugin);
        app
    }

    #[test]
    fn formats_position_with_two_decimal_places() {
        assert_eq!(
            format_position(Vec3::new(1.234, -2.0, 9.876)),
            "Position\nX: 1.23\nY: -2.00\nZ: 9.88"
        );
    }

    #[test]
    fn shows_controlled_player_position_during_gameplay() {
        let mut app = test_app();
        app.world_mut().resource_mut::<GameScreen>().0 = Screen::InGame;
        app.world_mut()
            .spawn((LocalPlayer, Position(Vec3::new(3.0, 1.5, -4.25))));

        app.update();

        let root = app
            .world_mut()
            .query_filtered::<&Node, With<DebugPositionUi>>()
            .single(app.world())
            .expect("debug position root");
        assert_eq!(root.display, Display::Flex);

        let text = app
            .world_mut()
            .query_filtered::<&Text, With<DebugPositionText>>()
            .single(app.world())
            .expect("debug position text");
        assert_eq!(text.0, "Position\nX: 3.00\nY: 1.50\nZ: -4.25");
    }

    #[test]
    fn stays_hidden_outside_gameplay() {
        let mut app = test_app();
        app.world_mut()
            .spawn((LocalPlayer, Position(Vec3::new(3.0, 1.5, -4.25))));

        app.update();

        let root = app
            .world_mut()
            .query_filtered::<&Node, With<DebugPositionUi>>()
            .single(app.world())
            .expect("debug position root");
        assert_eq!(root.display, Display::None);
    }
}
