//! Campo di testo modificabile da tastiera.
//!
//! Lo stato (valore, focus, errore) vive nel componente [`TextInput`]; i sistemi
//! UI centrali si occupano di rifletterlo sui nodi testo figli
//! ([`TextInputValueText`], [`TextInputErrorText`]) e di gestire la tastiera.

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
}

impl TextInput {
    pub fn new(placeholder: impl Into<String>, max_chars: usize) -> Self {
        Self {
            value: String::new(),
            focused: false,
            error: None,
            placeholder: placeholder.into(),
            max_chars,
        }
    }
}

/// Marker: testo che mostra il valore corrente (o il placeholder).
#[derive(Component)]
pub struct TextInputValueText;

/// Marker: testo che mostra l'errore di validazione corrente.
#[derive(Component)]
pub struct TextInputErrorText;

/// Genera un campo di testo focalizzabile attaccato a `parent`.
///
/// Ritorna l'entity del wrapper colonna (input + messaggio di errore).
pub fn spawn_text_input(
    commands: &mut Commands,
    parent: Entity,
    placeholder: impl Into<String>,
    max_chars: usize,
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
            TextInput::new(placeholder_str.clone(), max_chars),
        ))
        .id();
    commands.entity(wrapper).add_child(input_entity);

    let value_text = commands
        .spawn((
            Text::new(placeholder_str),
            TextFont {
                font_size: FontSize::Px(theme.input_font_size),
                ..default()
            },
            TextColor(theme.muted_text_color),
            TextInputValueText,
        ))
        .id();
    commands.entity(input_entity).add_child(value_text);

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
    commands.entity(wrapper).add_child(error_text);

    wrapper
}
