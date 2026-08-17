//! Campo di testo modificabile da tastiera.
//!
//! Lo stato (valore, focus, errore) vive nel componente [`TextInput`]; i sistemi
//! UI centrali si occupano di rifletterlo sui nodi testo figli
//! ([`TextInputValueText`], [`TextInputErrorText`]) e di gestire la tastiera.
//!
//! Più di un campo può esistere contemporaneamente (es. email + password nel
//! form di login): ogni [`TextInput`] porta i riferimenti diretti ai propri
//! nodi di testo, così i sistemi condivisi non devono presumere "ce n'è uno
//! solo" e possono scrivere sul nodo giusto per ciascuna entity.

use bevy::prelude::*;

use crate::ui::theme::UiTheme;

/// Campo di testo focalizzabile e modificabile da tastiera.
#[derive(Component)]
pub struct TextInput {
    pub value: String,
    pub focused: bool,
    pub error: Option<String>,
    pub placeholder: String,
    pub max_chars: usize,
    /// Se `true`, il valore è mostrato mascherato (campo password).
    pub obscured: bool,
    /// Entity del nodo di testo che mostra valore/placeholder.
    pub(crate) value_text: Entity,
    /// Entity del nodo di testo che mostra l'errore di validazione.
    pub(crate) error_text: Entity,
}

/// Marker: testo che mostra il valore corrente (o il placeholder).
#[derive(Component)]
pub struct TextInputValueText;

/// Marker: testo che mostra l'errore di validazione corrente.
#[derive(Component)]
pub struct TextInputErrorText;

/// Genera un campo di testo focalizzabile attaccato a `parent`.
///
/// Ritorna l'entity del campo stesso (quella che porta [`TextInput`]), utile
/// per aggiungere un marker che lo distingua da altri campi.
pub fn spawn_text_input(
    commands: &mut Commands,
    parent: Entity,
    placeholder: impl Into<String>,
    max_chars: usize,
    theme: &UiTheme,
) -> Entity {
    spawn_text_input_with_options(commands, parent, placeholder, max_chars, false, theme)
}

/// Come [`spawn_text_input`], ma il valore digitato è mostrato mascherato.
pub fn spawn_password_input(
    commands: &mut Commands,
    parent: Entity,
    placeholder: impl Into<String>,
    max_chars: usize,
    theme: &UiTheme,
) -> Entity {
    spawn_text_input_with_options(commands, parent, placeholder, max_chars, true, theme)
}

fn spawn_text_input_with_options(
    commands: &mut Commands,
    parent: Entity,
    placeholder: impl Into<String>,
    max_chars: usize,
    obscured: bool,
    theme: &UiTheme,
) -> Entity {
    let placeholder_str = placeholder.into();
    let wrapper = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(wrapper);

    // Spawned before `input_entity` so their ids can be stored directly on the
    // `TextInput` component — the shared systems that render/update a field
    // then write to exactly the right nodes, never a global "the one input".
    let value_text = commands
        .spawn((
            Text::new(placeholder_str.clone()),
            TextFont {
                font_size: FontSize::Px(theme.input_font_size),
                ..default()
            },
            TextColor(theme.muted_text_color),
            TextInputValueText,
        ))
        .id();

    let error_text = commands
        .spawn((
            Text::new(String::new()),
            TextFont {
                font_size: FontSize::Px(theme.input_font_size - 4.0),
                ..default()
            },
            TextColor(theme.error_color),
            TextInputErrorText,
        ))
        .id();

    let input_entity = commands
        .spawn((
            Button,
            Node {
                width: Val::Px(280.0),
                height: Val::Px(40.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(theme.input_bg),
            BorderColor::all(theme.input_border),
            TextInput {
                value: String::new(),
                focused: false,
                error: None,
                placeholder: placeholder_str,
                max_chars,
                obscured,
                value_text,
                error_text,
            },
        ))
        .id();

    commands.entity(wrapper).add_child(input_entity);
    commands.entity(input_entity).add_child(value_text);
    commands.entity(wrapper).add_child(error_text);

    input_entity
}
