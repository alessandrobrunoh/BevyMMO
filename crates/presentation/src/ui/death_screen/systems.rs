//! Sistemi della death screen: setup UI, visibilità, invio `RespawnRequest`.

use bevy::prelude::*;
use bevymmo_client::local_player::LocalPlayer;
use bevymmo_client::stdb::{commands, StdbConnection};

use bevymmo_client::network::types::ClientConnectionConfig;
use bevymmo_gameplay::entity::components::EntityState;
use bevymmo_gameplay::stats::components::VitalStats;
use bevymmo_network::network::protocol::PlayerId;

use crate::game_state::Screen;
use crate::ui::button::{apply_button_image, spawn_bar_button, UiButtonImages};
use crate::ui::text::spawn_text;
use crate::ui::theme::UiTheme;

use super::plugin::{DeathScreenButton, DeathScreenRoot};

/// Costruisce l'overlay una tantum (in `Startup`), nascosto di default.
pub fn setup_death_screen(mut commands: Commands, theme: Res<UiTheme>) {
    let backdrop = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(theme.panel_bg),
            DeathScreenRoot,
        ))
        .id();

    let panel = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(16.0),
            padding: UiRect::all(Val::Px(32.0)),
            ..default()
        })
        .id();
    commands.entity(backdrop).add_child(panel);

    spawn_text(
        &mut commands,
        panel,
        "You died",
        theme.title_font_size,
        theme.text_color,
    );
    spawn_text(
        &mut commands,
        panel,
        "Press Respawn to go back into the game",
        theme.button_font_size,
        theme.muted_text_color,
    );

    // Custom button: the action is a network message, not a `UiButtonAction`.
    // Visuals (hover/press) still go through `UiButtonImages` /
    // `update_respawn_button_visuals`.
    spawn_bar_button(&mut commands, panel, "Respawn", &theme, DeathScreenButton);
}

/// Mostra l'overlay solo quando il player locale è `Dead` e siamo in gameplay.
pub fn update_death_screen_visibility(
    screen: Res<State<Screen>>,
    client_config: Option<Res<ClientConnectionConfig>>,
    players: Query<(
        &EntityState,
        Option<&VitalStats>,
        Option<&PlayerId>,
        Has<LocalPlayer>,
    )>,
    mut roots: Query<&mut Node, With<DeathScreenRoot>>,
) {
    let local_client_id = client_config.as_deref().map(|c| c.client_id);
    let is_local_dead = local_player_state(&players, local_client_id)
        .map(|(state, vital)| state.is_dead() || vital.is_some_and(VitalStats::is_dead))
        .unwrap_or(false);
    let visible = *screen.get() == Screen::InGame && is_local_dead;

    let display = if visible {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in roots.iter_mut() {
        node.display = display;
    }
}

/// Invia un `RespawnRequest` al server quando il pulsante viene premuto.
///
/// Il server decide se la richiesta è valida (player Dead); eventuali click
/// multipli prima della replica vengono gestiti lato server come no-op.
pub fn handle_respawn_button(
    buttons: Query<&Interaction, (Changed<Interaction>, With<DeathScreenButton>)>,
    conn: Option<Res<StdbConnection>>,
) {
    let any_pressed = buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    if !any_pressed {
        return;
    }
    // Absent before the connection is established; the death screen cannot be
    // on screen then, but the option keeps the system from panicking if it is.
    let Some(conn) = conn else {
        return;
    };
    if let Err(err) = commands::respawn(&conn) {
        error!("respawn failed: {err}");
    }
}

/// Visual feedback (hover/press) per il pulsante Respawn, separato dal
/// `UiButton` centrale perché l'azione non è una `UiButtonAction`.
pub fn update_respawn_button_visuals(
    mut query: Query<
        (&Interaction, &mut ImageNode, &UiButtonImages),
        (With<DeathScreenButton>, Changed<Interaction>),
    >,
) {
    for (interaction, mut image, button_images) in query.iter_mut() {
        apply_button_image(*interaction, &mut image, button_images);
    }
}

fn local_player_state<'a>(
    players: &'a Query<(
        &EntityState,
        Option<&VitalStats>,
        Option<&PlayerId>,
        Has<LocalPlayer>,
    )>,
    local_client_id: Option<u64>,
) -> Option<(&'a EntityState, Option<&'a VitalStats>)> {
    // Prima cerca per `LocalPlayer` (player locale predetto), poi fallback su
    // `PlayerId == client_id` (player interpolato in single-player d'ospite).
    players
        .iter()
        .find(|(_, _, _, controlled)| *controlled)
        .or_else(|| {
            players.iter().find(|(_, _, player_id, _)| {
                player_id.is_some_and(|id| {
                    local_client_id.is_some_and(|client_id| id.0.to_bits() == client_id)
                })
            })
        })
        .map(|(state, vital, _, _)| (state, vital))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::{init_screen_states, Screen};
    use crate::ui::theme::UiTheme;
    use bevymmo_client::local_player::LocalPlayer;
    use bevymmo_gameplay::entity::components::EntityState;

    fn test_app() -> App {
        let mut app = App::new();
        app.init_resource::<UiTheme>();
        init_screen_states(&mut app);
        app.add_systems(Startup, setup_death_screen);
        app.add_systems(Update, update_death_screen_visibility);
        app
    }

    fn root_visibility(app: &mut App) -> Display {
        let mut query = app
            .world_mut()
            .query_filtered::<&Node, With<DeathScreenRoot>>();
        query.single(app.world()).expect("root").display
    }

    #[test]
    fn overlay_starts_hidden() {
        let mut app = test_app();
        app.update();
        assert_eq!(root_visibility(&mut app), Display::None);
    }

    #[test]
    fn respawn_button_uses_sliced_bar_art() {
        let mut app = test_app();
        app.update();

        let mut query = app
            .world_mut()
            .query_filtered::<&ImageNode, With<DeathScreenButton>>();
        let image = query.single(app.world()).expect("respawn button");
        assert!(matches!(image.image_mode, NodeImageMode::Sliced(_)));
    }

    #[test]
    fn overlay_shows_when_local_player_is_dead_in_game() {
        let mut app = test_app();
        app.insert_state(Screen::InGame);
        app.world_mut().spawn((LocalPlayer, EntityState::Dead));
        app.update();

        assert_eq!(root_visibility(&mut app), Display::Flex);
    }

    #[test]
    fn overlay_hides_when_player_respawns() {
        let mut app = test_app();
        app.insert_state(Screen::InGame);
        let player = app.world_mut().spawn((LocalPlayer, EntityState::Dead)).id();
        app.update();
        assert_eq!(root_visibility(&mut app), Display::Flex);

        *app.world_mut()
            .entity_mut(player)
            .get_mut::<EntityState>()
            .unwrap() = EntityState::Idle;
        app.update();
        assert_eq!(root_visibility(&mut app), Display::None);
    }

    #[test]
    fn overlay_hides_when_leaving_gameplay() {
        let mut app = test_app();
        app.insert_state(Screen::InGame);
        app.world_mut().spawn((LocalPlayer, EntityState::Dead));
        app.update();
        assert_eq!(root_visibility(&mut app), Display::Flex);

        app.insert_state(Screen::MainMenu);
        app.update();
        assert_eq!(root_visibility(&mut app), Display::None);
    }

    #[test]
    fn overlay_stays_hidden_when_local_player_is_alive() {
        let mut app = test_app();
        app.insert_state(Screen::InGame);
        app.world_mut().spawn((LocalPlayer, EntityState::Idle));
        app.update();
        assert_eq!(root_visibility(&mut app), Display::None);
    }
}
