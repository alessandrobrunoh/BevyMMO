use bevy::prelude::*;

use bevymmo_shared::entity::components::PlayerName;
use bevymmo_shared::user_settings::{GameSettingsResource, KeyAction};

use crate::ui::theme::UiTheme;

use super::plugin::{ScoreboardPanel, ScoreboardState, ScoreboardUi};
use crate::ui::text::spawn_text;

pub fn setup_scoreboard(mut commands: Commands, theme: Res<UiTheme>) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            ScoreboardUi,
        ))
        .id();

    let panel = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(theme.panel_bg),
            ScoreboardPanel,
        ))
        .id();

    commands.entity(root).add_child(panel);
}

/// Aggiorna visibilità e contenuto della scoreboard.
///
/// Per evitare clonaggi/ordinamenti di `Vec<String>` ogni frame, la lista nomi
/// viene ricostruita solo quando si verifica uno di questi eventi:
/// - apertura del pannello (`just_opened`);
/// - join di un nuovo player (`Added<PlayerName>`, coperto da `Changed`);
/// - rename di un player esistente (`Changed<PlayerName>`);
/// - leave di un player (il numero di `PlayerName` cambia).
///
/// Negli altri frame il work è solo un `count()` (nessuna allocazione) e un
/// controllo su `Changed<PlayerName>` (O(1) in assenza di modifiche). La lista in
/// [`ScoreboardState`] resta stabile.
pub fn update_scoreboard(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<GameSettingsResource>,
    theme: Res<UiTheme>,
    players: Query<&PlayerName>,
    changed_names: Query<(), Changed<PlayerName>>,
    mut state: ResMut<ScoreboardState>,
    mut scoreboard_query: Query<&mut Node, With<ScoreboardUi>>,
    panel_query: Query<Entity, With<ScoreboardPanel>>,
) {
    let Ok(mut root_node) = scoreboard_query.single_mut() else {
        return;
    };
    let Ok(panel) = panel_query.single() else {
        return;
    };

    let open = settings.pressed(KeyAction::ShowScoreboard, &keys);
    root_node.display = if open { Display::Flex } else { Display::None };

    if !open {
        state.open = false;
        return;
    }

    let just_opened = !state.open;

    // Verifica dirty a costo costante per frame:
    // - `Changed<PlayerName>` copre Added (join) e rename;
    // - un cambiamento del numero di player copre i leave (despawn).
    let name_count = players.iter().count();
    let dirty = just_opened || name_count != state.names.len() || !changed_names.is_empty();

    state.open = true;

    if !dirty {
        return;
    }

    let mut current: Vec<String> = players.iter().map(|p| p.0.clone()).collect();
    // Ordina per confronto indipendente dall'ordine di iterazione.
    current.sort();
    state.names = current;

    commands.entity(panel).despawn_related::<Children>();
    spawn_text(
        &mut commands,
        panel,
        "Connected Clients",
        theme.scoreboard_title_size,
        theme.text_color,
    );
    for name in &state.names {
        spawn_text(
            &mut commands,
            panel,
            name.clone(),
            theme.scoreboard_entry_size,
            theme.muted_text_color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::{GameScreen, Screen};
    use crate::ui::scoreboard::ScoreboardPlugin;
    use bevy::input::InputPlugin;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(InputPlugin);
        app.init_resource::<UiTheme>();
        app.init_resource::<GameScreen>();
        app.insert_resource(GameSettingsResource(
            bevymmo_shared::user_settings::GameSettings::default(),
        ));
        app.add_plugins(ScoreboardPlugin);
        app.world_mut().resource_mut::<GameScreen>().0 = Screen::InGame;
        app
    }

    fn press_tab(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Tab);
    }

    fn panel_first_child(app: &mut App) -> Option<Entity> {
        let panel = app
            .world_mut()
            .query_filtered::<Entity, With<ScoreboardPanel>>()
            .single(app.world())
            .ok()?;
        app.world()
            .entity(panel)
            .get::<Children>()
            .and_then(|c| c.first().copied())
    }

    #[test]
    fn opening_lists_current_players_sorted() {
        let mut app = test_app();
        app.world_mut().spawn(PlayerName("Bob".to_string()));
        app.world_mut().spawn(PlayerName("Alice".to_string()));
        // Startup frame; scoreboard ancora chiusa (Tab non premuto).
        app.update();

        press_tab(&mut app);
        app.update();

        let state = app.world().resource::<ScoreboardState>();
        assert!(state.open);
        assert_eq!(state.names, vec!["Alice", "Bob"]);
    }

    #[test]
    fn closing_does_not_keep_state_open() {
        let mut app = test_app();
        app.world_mut().spawn(PlayerName("Alice".to_string()));
        app.update();

        press_tab(&mut app);
        app.update();
        assert!(app.world().resource::<ScoreboardState>().open);

        // Rilascia Tab: il pannello si chiude e lo stato segue.
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::Tab);
        app.update();

        assert!(!app.world().resource::<ScoreboardState>().open);
    }

    #[test]
    fn does_not_rebuild_when_nothing_changes() {
        let mut app = test_app();
        app.world_mut().spawn(PlayerName("Alice".to_string()));
        press_tab(&mut app);
        app.update();

        let first_child_after_open = panel_first_child(&mut app).expect("panel has title text");

        // Frame idle: Tab ancora premuto, nessuna modifica ai PlayerName.
        for _ in 0..5 {
            app.update();
        }

        let first_child_after_idle =
            panel_first_child(&mut app).expect("panel still has title text");
        assert_eq!(
            first_child_after_open, first_child_after_idle,
            "i figli del pannello devono restare stabili senza rebuild"
        );
        assert_eq!(
            app.world().resource::<ScoreboardState>().names,
            vec!["Alice"]
        );
    }

    #[test]
    fn rebuilds_on_join() {
        let mut app = test_app();
        app.world_mut().spawn(PlayerName("Alice".to_string()));
        press_tab(&mut app);
        app.update();
        app.update(); // consuma l'Added iniziale

        let before = panel_first_child(&mut app);
        app.world_mut().spawn(PlayerName("Bob".to_string()));
        app.update();

        let state = app.world().resource::<ScoreboardState>();
        assert_eq!(state.names, vec!["Alice", "Bob"]);
        assert_ne!(
            panel_first_child(&mut app),
            before,
            "il pannello deve essere stato rebuildato sul join"
        );
    }

    #[test]
    fn rebuilds_on_leave() {
        let mut app = test_app();
        let bob = app.world_mut().spawn(PlayerName("Bob".to_string())).id();
        app.world_mut().spawn(PlayerName("Alice".to_string()));
        press_tab(&mut app);
        app.update();
        app.update(); // consuma l'Added iniziale

        let before = panel_first_child(&mut app);
        app.world_mut().entity_mut(bob).despawn();
        app.update();

        let state = app.world().resource::<ScoreboardState>();
        assert_eq!(state.names, vec!["Alice"]);
        assert_ne!(
            panel_first_child(&mut app),
            before,
            "il pannello deve essere stato rebuildato sul leave"
        );
    }

    #[test]
    fn rebuilds_on_rename() {
        let mut app = test_app();
        let alice = app.world_mut().spawn(PlayerName("Alice".to_string())).id();
        press_tab(&mut app);
        app.update();
        app.update(); // consuma l'Added iniziale

        let before = panel_first_child(&mut app);
        app.world_mut()
            .entity_mut(alice)
            .get_mut::<PlayerName>()
            .unwrap()
            .0 = "Alicia".to_string();
        app.update();

        let state = app.world().resource::<ScoreboardState>();
        assert_eq!(state.names, vec!["Alicia"]);
        assert_ne!(
            panel_first_child(&mut app),
            before,
            "il pannello deve essere stato rebuildato sul rename"
        );
    }

    #[test]
    fn leave_and_join_same_frame_with_same_count_still_rebuilds() {
        let mut app = test_app();
        let bob = app.world_mut().spawn(PlayerName("Bob".to_string())).id();
        app.world_mut().spawn(PlayerName("Alice".to_string()));
        press_tab(&mut app);
        app.update();
        app.update(); // consuma l'Added iniziale

        let before = panel_first_child(&mut app);

        // Sostituisce Bob con Carol nello stesso frame: count resta 2 ma il
        // set di nomi cambia. Changed<PlayerName> su Carol (Added) forza il
        // rebuild anche se la lunghezza non è cambiata.
        app.world_mut().entity_mut(bob).despawn();
        app.world_mut().spawn(PlayerName("Carol".to_string()));
        app.update();

        let state = app.world().resource::<ScoreboardState>();
        assert_eq!(state.names, vec!["Alice", "Carol"]);
        assert_ne!(
            panel_first_child(&mut app),
            before,
            "il pannello deve rebuildare anche a parità di count"
        );
    }
}
