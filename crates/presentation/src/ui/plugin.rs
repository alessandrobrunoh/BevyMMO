//! Registrazione del plugin UI e della camera dedicata.

use bevy::prelude::*;

use super::{
    boss_bar, card, connecting, crowd_control_bar, death_screen, debug_position, entity_bar,
    inscription, inventory, main_menu, notices, npc_sidebar, pause_menu, player_stats, scoreboard,
    scrollbar, settings, spell_selector, status_bar, systems, target_frame, target_indicator,
};

use crate::ui::theme::UiTheme;

/// Camera 2D dedicata alla UI. Resta attiva nel menu e durante la partita,
/// sopra la camera 3D della scena gameplay.
#[derive(Component)]
struct UiCamera;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiTheme>();
        app.add_systems(Startup, setup_ui_camera);
        app.add_plugins((
            card::CardPlugin,
            entity_bar::EntityBarPlugin,
            scoreboard::ScoreboardPlugin,
            main_menu::MainMenuPlugin,
            settings::SettingsPlugin,
            pause_menu::PauseMenuPlugin,
            player_stats::PlayerStatsPlugin,
            connecting::ConnectingPlugin,
        ));
        app.add_plugins((
            target_indicator::TargetIndicatorPlugin,
            target_frame::TargetFramePlugin,
            death_screen::DeathScreenPlugin,
            crowd_control_bar::CrowdControlBarPlugin,
            spell_selector::SpellSelectorUiPlugin,
            inscription::InscriptionUiPlugin,
            inventory::InventoryUiPlugin,
            boss_bar::BossBarPlugin,
            status_bar::StatusBarPlugin,
            scrollbar::ScrollbarPlugin,
            notices::NoticesPlugin,
            npc_sidebar::NpcSidebarPlugin,
        ));
        app.add_plugins(debug_position::DebugPositionPlugin);

        app.add_systems(
            Update,
            (
                systems::update_button_actions,
                systems::update_button_visuals,
                systems::update_text_input_focus,
                systems::update_text_input_keyboard,
                systems::update_text_input_display,
                systems::update_connection_failure,
                systems::toggle_pause,
            ),
        );
    }
}

fn setup_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        UiCamera,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_camera_is_created_above_the_game_camera_without_clearing_it() {
        let mut app = App::new();
        app.add_systems(Startup, setup_ui_camera);
        app.update();

        let mut cameras = app.world_mut().query_filtered::<&Camera, With<UiCamera>>();
        let camera = cameras.single(app.world()).expect("one UI camera");
        assert_eq!(camera.order, 1);
        assert!(matches!(camera.clear_color, ClearColorConfig::None));
    }
}
