//! Pannello con le statistiche del Player locale.

use bevy::prelude::*;

use super::systems::{setup_player_stats, update_player_stats};

/// Marker del nodo root del pannello stats.
#[derive(Component)]
pub struct PlayerStatsUi;

/// Marker del testo aggiornato dal sistema stats.
#[derive(Component)]
pub struct PlayerStatsText;

pub struct PlayerStatsPlugin;

impl Plugin for PlayerStatsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player_stats);
        app.add_systems(
            Update,
            update_player_stats.run_if(crate::ui::systems::in_gameplay),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::{GameScreen, Screen};
    use crate::plugins::entity::components::Stats;
    use crate::ui::theme::UiTheme;
    use lightyear::prelude::Controlled;

    fn test_app() -> App {
        let mut app = App::new();
        app.init_resource::<UiTheme>();
        app.init_resource::<GameScreen>();
        app.add_plugins(PlayerStatsPlugin);
        app.world_mut().resource_mut::<GameScreen>().0 = Screen::InGame;
        app
    }

    fn panel_text(app: &mut App) -> String {
        let text_entity = app
            .world_mut()
            .query_filtered::<Entity, With<PlayerStatsText>>()
            .single(app.world())
            .expect("stats text");
        app.world()
            .entity(text_entity)
            .get::<Text>()
            .expect("Text component")
            .0
            .clone()
    }

    #[test]
    fn shows_local_player_stats_in_the_top_right_panel() {
        let mut app = test_app();
        app.world_mut().spawn((
            Controlled,
            Stats::with_combat_values(0.15, 10.0, 100.0, 80.0, 4.0, 100.0),
        ));
        app.update();

        let root = app
            .world_mut()
            .query_filtered::<&Node, With<PlayerStatsUi>>()
            .single(app.world())
            .expect("stats root");

        assert_eq!(root.position_type, PositionType::Absolute);
        assert_eq!(root.right, Val::Px(16.0));
        assert_eq!(root.top, Val::Px(16.0));
        assert_eq!(panel_text(&mut app), "Max HP: 100\nMax Mana: 80\nMana Regen: 4.0/s\nArmor: 100 (50% reduction)");
    }

    #[test]
    fn updates_when_local_player_stats_change_and_hides_outside_gameplay() {
        let mut app = test_app();
        let player = app
            .world_mut()
            .spawn((
                Controlled,
                Stats::with_combat_values(0.15, 10.0, 100.0, 80.0, 4.0, 0.0),
            ))
            .id();
        app.update();

        app.world_mut().entity_mut(player).get_mut::<Stats>().unwrap().max_mana = 120.0;
        app.world_mut().resource_mut::<GameScreen>().0 = Screen::MainMenu;
        app.update();

        let root = app
            .world_mut()
            .query_filtered::<&Node, With<PlayerStatsUi>>()
            .single(app.world())
            .expect("stats root");
        assert_eq!(root.display, Display::None);

        app.world_mut().resource_mut::<GameScreen>().0 = Screen::InGame;
        app.update();
        assert_eq!(panel_text(&mut app), "Max HP: 100\nMax Mana: 120\nMana Regen: 4.0/s\nArmor: 0 (0% reduction)");
    }
}
