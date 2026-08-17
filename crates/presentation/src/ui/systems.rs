//! Sistemi UI condivisi tra menu, settings e pause overlay.
//!
//! La visibilità di ciascuna schermata viene gestita con un cambio di
//! [`Display`] sul nodo root, non con respawn: lo spawn avviene una volta in
//! `Startup`.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonInput;
use bevy::input::ButtonState;
use bevy::prelude::*;

use bevymmo_client::user_settings::{GameSettingsResource, KeyAction};

use crate::game_state::{
    validate_email, validate_password, validate_player_name, AuthIntent, AuthRequest,
    ConnectionFailure, ConnectionIntent, ConnectionRequest, EmailError, GameScreen,
    PasswordError, PlayerNameError, Screen,
};
use crate::ui::button::{UiButton, UiButtonAction, UiButtonImages};
use crate::ui::login::{EmailInput, PasswordInput};
use crate::ui::main_menu::PlayerNameInput;
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

fn email_error_message(err: EmailError) -> String {
    match err {
        EmailError::MissingAt => "Email must contain '@'.".to_string(),
        EmailError::EmptyLocalOrDomain => "Email must have text before and after '@'.".to_string(),
        EmailError::DomainMissingDot => "Email domain must contain a '.'.".to_string(),
    }
}

fn password_error_message(err: PasswordError) -> String {
    match err {
        PasswordError::TooShort => "Password must be at least 8 characters.".to_string(),
    }
}

/// Dispatch delle azioni associate ai pulsanti UI.
///
/// Legge solo i pulsanti il cui [`Interaction`] è cambiato ed è `Pressed`.
pub fn update_button_actions(
    mut screen: ResMut<GameScreen>,
    mut connection_request: ResMut<ConnectionRequest>,
    buttons: Query<(&Interaction, &UiButton), Changed<Interaction>>,
    mut name_input: Query<&mut TextInput, With<PlayerNameInput>>,
) {
    for (interaction, button) in buttons.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match button.action {
            UiButtonAction::Play => {
                let Ok(mut input) = name_input.single_mut() else {
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
            // Handled by `update_auth_button_actions`, which needs the
            // email/password fields this system does not query.
            UiButtonAction::Login | UiButtonAction::Register => {}
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
                // Goes through `stdb::plugin::begin_shutdown`/`finish_shutdown`
                // rather than writing `AppExit` directly, so the pending
                // disconnect actually reaches the socket before the process
                // dies. See `ConnectionIntent::Shutdown`.
                connection_request.0 = Some(ConnectionIntent::Shutdown);
            }
            // Handled by `settings::systems::reset_keybinds_on_button`.
            UiButtonAction::ResetKeybinds => {}
        }
    }
}

/// Dispatch delle azioni Login/Register del form di autenticazione.
///
/// Separato da [`update_button_actions`] perché legge due campi (email,
/// password) invece di uno solo, e nessun'altra azione ne ha bisogno.
pub fn update_auth_button_actions(
    mut auth_request: ResMut<AuthRequest>,
    buttons: Query<(&Interaction, &UiButton), Changed<Interaction>>,
    mut email_input: Query<&mut TextInput, (With<EmailInput>, Without<PasswordInput>)>,
    mut password_input: Query<&mut TextInput, (With<PasswordInput>, Without<EmailInput>)>,
) {
    for (interaction, button) in buttons.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let is_register = match button.action {
            UiButtonAction::Login => false,
            UiButtonAction::Register => true,
            _ => continue,
        };

        let Ok(mut email) = email_input.single_mut() else {
            continue;
        };
        let Ok(mut password) = password_input.single_mut() else {
            continue;
        };

        let email_result = validate_email(&email.value);
        let password_result = validate_password(&password.value);
        email.error = email_result.clone().err().map(email_error_message);
        password.error = password_result.clone().err().map(password_error_message);

        if let (Ok(normalized_email), Ok(())) = (email_result, password_result) {
            let password_value = password.value.clone();
            auth_request.0 = Some(if is_register {
                AuthIntent::Register {
                    email: normalized_email,
                    password: password_value,
                }
            } else {
                AuthIntent::Login {
                    email: normalized_email,
                    password: password_value,
                }
            });
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

/// Focalizza il campo cliccato, sfocalizzando ogni altro campo aperto.
///
/// Più campi possono esistere insieme (es. email + password nel login): al
/// più uno è focalizzato alla volta, altrimenti la tastiera non saprebbe a
/// quale campo indirizzare gli eventi.
pub fn update_text_input_focus(
    clicked: Query<(Entity, &Interaction), (With<TextInput>, Changed<Interaction>)>,
    mut inputs: Query<(Entity, &mut TextInput)>,
) {
    let Some(clicked_entity) = clicked
        .iter()
        .find(|(_, interaction)| **interaction == Interaction::Pressed)
        .map(|(entity, _)| entity)
    else {
        return;
    };
    for (entity, mut input) in inputs.iter_mut() {
        input.focused = entity == clicked_entity;
    }
}

/// Gestione tastiera del campo di testo attualmente focalizzato, se c'è.
pub fn update_text_input_keyboard(
    mut events: MessageReader<KeyboardInput>,
    mut query: Query<&mut TextInput>,
) {
    let Some(mut input) = query.iter_mut().find(|input| input.focused) else {
        events.clear();
        return;
    };

    let len = input.value.chars().count();
    for ev in events.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        match &ev.logical_key {
            Key::Backspace => {
                input.value.pop();
            }
            Key::Enter | Key::Escape => {
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

/// Riflette lo stato di ogni [`TextInput`] cambiato sui propri nodi testo
/// figli (valore/placeholder ed errore) e sul proprio bordo.
///
/// Scrive tramite gli entity id salvati su `TextInput` (`value_text`,
/// `error_text`), non tramite una ricerca globale: con più campi presenti
/// contemporaneamente non esiste "il" nodo valore, solo il nodo di *questo*
/// campo.
pub fn update_text_input_display(
    theme: Res<UiTheme>,
    query: Query<(Entity, &TextInput), Changed<TextInput>>,
    mut value_text: Query<(&mut Text, &mut TextColor), With<TextInputValueText>>,
    mut error_text: Query<&mut Text, (With<TextInputErrorText>, Without<TextInputValueText>)>,
    mut border: Query<&mut BorderColor>,
) {
    for (entity, input) in query.iter() {
        if let Ok((mut text, mut color)) = value_text.get_mut(input.value_text) {
            if input.value.is_empty() {
                text.0 = input.placeholder.clone();
                color.0 = theme.muted_text_color;
            } else {
                text.0 = if input.obscured {
                    "•".repeat(input.value.chars().count())
                } else {
                    input.value.clone()
                };
                color.0 = theme.text_color;
            }
        }

        if let Ok(mut text) = error_text.get_mut(input.error_text) {
            text.0 = input.error.clone().unwrap_or_default();
        }

        if let Ok(mut border_color) = border.get_mut(entity) {
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
    use bevymmo_client::user_settings::{GameSettings, KeyBinding, KeyModifiers};

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
