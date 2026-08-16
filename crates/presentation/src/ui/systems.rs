//! Sistemi UI condivisi tra menu, settings e pause overlay.
//!
//! La visibilità di ciascuna schermata viene gestita con un cambio di
//! [`Display`] sul nodo root, non con respawn: lo spawn avviene una volta in
//! `Startup`.

use bevy::app::AppExit;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonInput;
use bevy::input::ButtonState;
use bevy::prelude::*;

use bevymmo_shared::user_settings::{GameSettingsResource, KeyAction};

use crate::game_state::{
    validate_player_name, ConnectionFailure, ConnectionIntent, ConnectionRequest, GameScreen,
    PlayerNameError, Screen,
};
use crate::ui::button::{UiButton, UiButtonAction, UiButtonImages};
use crate::ui::text_input::{TextInput, TextInputErrorText, TextInputValueText};
use crate::ui::theme::UiTheme;

/// Condizione di esecuzione: il client è in una schermata di gameplay.
pub fn in_gameplay(screen: Res<GameScreen>) -> bool {
    matches!(screen.0, Screen::InGame | Screen::Paused)
}

fn error_message(err: PlayerNameError) -> String {
    match err {
        PlayerNameError::TooShort => "Name must be at least 3 characters.".to_string(),
        PlayerNameError::TooLong => "Name must be at most 16 characters.".to_string(),
    }
}

/// Dispatch delle azioni associate ai pulsanti UI.
///
/// Legge solo i pulsanti il cui [`Interaction`] è cambiato ed è `Pressed`.
pub fn update_button_actions(
    mut screen: ResMut<GameScreen>,
    mut connection_request: ResMut<ConnectionRequest>,
    mut exit: MessageWriter<AppExit>,
    buttons: Query<(&Interaction, &UiButton), Changed<Interaction>>,
    mut text_input: Query<&mut TextInput>,
) {
    for (interaction, button) in buttons.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match button.action {
            UiButtonAction::Play => {
                let Ok(mut input) = text_input.single_mut() else {
                    continue;
                };
                match validate_player_name(&input.value) {
                    Ok(name) => {
                        input.error = None;
                        input.focused = false;
                        screen.0 = Screen::Connecting;
                        connection_request.0 =
                            Some(ConnectionIntent::Connect { player_name: name });
                    }
                    Err(err) => {
                        input.error = Some(error_message(err));
                    }
                }
            }
            UiButtonAction::OpenSettings => {
                screen.0 = Screen::Settings;
            }
            UiButtonAction::BackToMenu => {
                screen.0 = Screen::MainMenu;
            }
            UiButtonAction::ReturnToMainMenu => {
                connection_request.0 = Some(ConnectionIntent::Disconnect);
                screen.0 = Screen::MainMenu;
            }
            UiButtonAction::Logout => {
                connection_request.0 = Some(ConnectionIntent::Logout);
                screen.0 = Screen::MainMenu;
            }
            UiButtonAction::Resume => {
                screen.0 = Screen::InGame;
            }
            UiButtonAction::Exit => {
                exit.write(AppExit::Success);
            }
            // Handled by `settings::systems::reset_keybinds_on_button`.
            UiButtonAction::ResetKeybinds => {}
        }
    }
}

/// Aggiorna la texture in base allo stato di interazione.
pub fn update_button_visuals(
    mut query: Query<
        (&Interaction, &mut ImageNode, &UiButtonImages),
        (With<UiButton>, Changed<Interaction>),
    >,
) {
    for (interaction, mut image, button_images) in query.iter_mut() {
        image.image = match interaction {
            Interaction::None => button_images.default.clone(),
            Interaction::Hovered => button_images.hover.clone(),
            Interaction::Pressed => button_images.clicked.clone(),
        };
    }
}

/// Toggle del focus sul click del campo di testo.
pub fn update_text_input_focus(
    mut query: Query<(&Interaction, &mut TextInput), Changed<Interaction>>,
) {
    for (interaction, mut input) in query.iter_mut() {
        if *interaction == Interaction::Pressed {
            input.focused = !input.focused;
        }
    }
}

/// Gestione tastiera del campo di testo quando è focalizzato.
pub fn update_text_input_keyboard(
    mut events: MessageReader<KeyboardInput>,
    mut query: Query<&mut TextInput>,
) {
    let Ok(mut input) = query.single_mut() else {
        return;
    };
    if !input.focused {
        events.clear();
        return;
    }

    let len = input.value.chars().count();
    for ev in events.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        match &ev.logical_key {
            Key::Backspace => {
                input.value.pop();
            }
            Key::Enter => {
                input.focused = false;
            }
            Key::Space if len < input.max_chars => {
                input.value.push(' ');
            }
            Key::Character(s) if len < input.max_chars => {
                // Un evento KeyboardInput può trasportare più di un carattere in
                // casi rari; prendiamo solo il primo stampabile ASCII.
                if let Some(ch) = s.chars().next().filter(|c| c.is_ascii_graphic()) {
                    input.value.push(ch);
                }
            }
            _ => {}
        }
    }
}

/// Riflette lo stato di [`TextInput`] sui nodi testo figli (valore/placeholder
/// ed errore) e sul bordo in base a focus/errore.
pub fn update_text_input_display(
    theme: Res<UiTheme>,
    query: Query<&TextInput, Changed<TextInput>>,
    mut value_text: Query<(&mut Text, &mut TextColor), With<TextInputValueText>>,
    mut error_text: Query<&mut Text, (With<TextInputErrorText>, Without<TextInputValueText>)>,
    mut border: Query<&mut BorderColor, With<TextInput>>,
) {
    let Ok(input) = query.single() else {
        return;
    };

    if let Ok((mut text, mut color)) = value_text.single_mut() {
        if input.value.is_empty() {
            text.0 = input.placeholder.clone();
            color.0 = theme.muted_text_color;
        } else {
            text.0 = input.value.clone();
            color.0 = theme.text_color;
        }
    }

    if let Ok(mut text) = error_text.single_mut() {
        text.0 = input.error.clone().unwrap_or_default();
    }

    if let Ok(mut border_color) = border.single_mut() {
        let color = if input.error.is_some() {
            theme.error_color
        } else if input.focused {
            theme.input_border_focused
        } else {
            theme.input_border
        };
        border_color.set_all(color);
    }
}

/// Aggiorna il testo che mostra l'eventuale errore di connessione nel menu
/// principale, leggendolo da [`ConnectionFailure`]. Non tocca
/// [`TextInput::error`] (validazione nome): i due canali sono indipendenti.
pub fn update_connection_failure(
    failure: Res<ConnectionFailure>,
    mut query: Query<&mut Text, With<crate::ui::main_menu::MainMenuConnectionFailure>>,
) {
    let Ok(mut text) = query.single_mut() else {
        return;
    };
    text.0 = failure.0.clone().unwrap_or_default();
}

/// Mostra/nasconde il pause overlay con the configured `TogglePause` key,
/// only in `InGame`/`Paused`.
///
/// Non tocca `Time`, `FixedUpdate` o la rete.
pub fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<GameSettingsResource>,
    mut screen: ResMut<GameScreen>,
) {
    if !settings.just_pressed(KeyAction::TogglePause, &keys) {
        return;
    }
    match screen.0 {
        Screen::InGame => screen.0 = Screen::Paused,
        Screen::Paused => screen.0 = Screen::InGame,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_shared::user_settings::{GameSettings, KeyBinding, KeyModifiers};

    fn test_app() -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<GameScreen>();
        app.insert_resource(GameSettingsResource(GameSettings::default()));
        app.add_systems(Update, toggle_pause);
        app
    }

    fn press(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
    }

    #[test]
    fn pause_uses_default_escape_binding() {
        let mut app = test_app();
        app.world_mut().resource_mut::<GameScreen>().0 = Screen::InGame;

        press(&mut app, KeyCode::KeyP);
        app.update();
        assert_eq!(app.world().resource::<GameScreen>().0, Screen::InGame);

        press(&mut app, KeyCode::Escape);
        app.update();
        assert_eq!(app.world().resource::<GameScreen>().0, Screen::Paused);
    }

    #[test]
    fn pause_respects_custom_binding() {
        let mut app = test_app();
        app.world_mut().resource_mut::<GameScreen>().0 = Screen::InGame;
        app.world_mut()
            .resource_mut::<GameSettingsResource>()
            .0
            .keybinds
            .bindings
            .insert(
                KeyAction::TogglePause,
                KeyBinding {
                    key: KeyCode::KeyP,
                    modifiers: KeyModifiers::default(),
                },
            );

        press(&mut app, KeyCode::Escape);
        app.update();
        assert_eq!(app.world().resource::<GameScreen>().0, Screen::InGame);

        press(&mut app, KeyCode::KeyP);
        app.update();
        assert_eq!(app.world().resource::<GameScreen>().0, Screen::Paused);
    }

    #[test]
    fn logout_sets_connection_intent_and_transitions_to_main_menu() {
        use crate::ui::button::{UiButton, UiButtonAction};

        let mut app = App::new();
        app.init_resource::<GameScreen>();
        app.init_resource::<ConnectionRequest>();
        app.insert_resource(GameSettingsResource(GameSettings::default()));

        // Spawn a button with Logout action
        let button_entity = app
            .world_mut()
            .spawn((
                UiButton {
                    action: UiButtonAction::Logout,
                },
                Interaction::Pressed,
            ))
            .id();

        app.add_systems(Update, update_button_actions);
        app.update();

        // Verify screen transitioned to MainMenu
        assert_eq!(app.world().resource::<GameScreen>().0, Screen::MainMenu);

        // Verify connection request was set to Logout
        let connection_request = app.world().resource::<ConnectionRequest>();
        assert!(connection_request.0.is_some());
        assert!(matches!(
            connection_request.0.as_ref().unwrap(),
            ConnectionIntent::Logout
        ));

        // Cleanup
        app.world_mut().despawn(button_entity);
    }
}
