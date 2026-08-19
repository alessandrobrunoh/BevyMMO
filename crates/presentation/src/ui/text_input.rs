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

/// Idle user/email field (baked user icon on the left cap).
pub const INPUT_GOLD_PATH: &str = "ui/extracted_kit/input_gold.png";
/// Focused / filled user field.
pub const INPUT_BLUE_PATH: &str = "ui/extracted_kit/input_blue.png";
/// Idle password field (baked lock + eye).
pub const INPUT_PASSWORD_GOLD_PATH: &str = "ui/extracted_kit/input_password_gold.png";
/// Focused / filled password field.
pub const INPUT_PASSWORD_BLUE_PATH: &str = "ui/extracted_kit/input_password_blue.png";

const INPUT_HEIGHT: f32 = 58.0;
const INPUT_MAX_WIDTH: f32 = 380.0;
/// Source-pixel 9-slice. The baked user/lock icon lives in the left ~160 px
/// of a 687 px bar — a 48 px slice stretched that icon across the field.
const INPUT_SLICE_LEFT: f32 = 168.0;
const INPUT_SLICE_RIGHT: f32 = 96.0;
const INPUT_SLICE_Y: f32 = 30.0;
/// Matches the rendered left cap at this height (`168 * 58/146`).
const INPUT_PAD_LEFT: f32 = 72.0;
const INPUT_PAD_RIGHT: f32 = 18.0;
/// Clears the baked eye on password bars.
const PASSWORD_PAD_RIGHT: f32 = 44.0;

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

/// Idle / focused bar textures for a [`TextInput`].
///
/// [`crate::ui::systems::update_text_input_display`] swaps [`ImageNode`] when
/// `focused` changes. Tests that spawn a bare [`TextInput`] omit this.
#[derive(Component, Clone)]
pub struct TextInputImages {
    pub idle: Handle<Image>,
    pub focused: Handle<Image>,
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
    asset_server: &AssetServer,
) -> Entity {
    spawn_text_input_with_options(
        commands,
        parent,
        placeholder,
        max_chars,
        false,
        theme,
        asset_server,
    )
}

/// Come [`spawn_text_input`], ma il valore digitato è mostrato mascherato.
pub fn spawn_password_input(
    commands: &mut Commands,
    parent: Entity,
    placeholder: impl Into<String>,
    max_chars: usize,
    theme: &UiTheme,
    asset_server: &AssetServer,
) -> Entity {
    spawn_text_input_with_options(
        commands,
        parent,
        placeholder,
        max_chars,
        true,
        theme,
        asset_server,
    )
}

fn spawn_text_input_with_options(
    commands: &mut Commands,
    parent: Entity,
    placeholder: impl Into<String>,
    max_chars: usize,
    obscured: bool,
    theme: &UiTheme,
    asset_server: &AssetServer,
) -> Entity {
    let placeholder_str = placeholder.into();
    let images = if obscured {
        TextInputImages {
            idle: asset_server.load(INPUT_PASSWORD_GOLD_PATH),
            focused: asset_server.load(INPUT_PASSWORD_BLUE_PATH),
        }
    } else {
        TextInputImages {
            idle: asset_server.load(INPUT_GOLD_PATH),
            focused: asset_server.load(INPUT_BLUE_PATH),
        }
    };
    let pad_right = if obscured {
        PASSWORD_PAD_RIGHT
    } else {
        INPUT_PAD_RIGHT
    };

    let wrapper = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            max_width: Val::Px(INPUT_MAX_WIDTH),
            min_width: Val::Px(0.0),
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
                font_size: FontSize::Px(theme.input_font_size - 2.0),
                ..default()
            },
            TextColor(theme.muted_text_color),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
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
                width: Val::Percent(100.0),
                max_width: Val::Px(INPUT_MAX_WIDTH),
                min_width: Val::Px(0.0),
                height: Val::Px(INPUT_HEIGHT),
                padding: UiRect::new(
                    Val::Px(INPUT_PAD_LEFT),
                    Val::Px(pad_right),
                    Val::Px(0.0),
                    Val::Px(0.0),
                ),
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                flex_shrink: 0.0,
                ..default()
            },
            sliced_input_image(images.idle.clone()),
            images,
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

/// 9-sliced input bar. Icon/lock caps stay intact; only the empty field stretches.
pub fn sliced_input_image(image: Handle<Image>) -> ImageNode {
    ImageNode::new(image).with_mode(NodeImageMode::Sliced(TextureSlicer {
        border: BorderRect::from([
            INPUT_SLICE_LEFT,
            INPUT_SLICE_RIGHT,
            INPUT_SLICE_Y,
            INPUT_SLICE_Y,
        ]),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    }))
}

/// Clears keyboard focus on every [`TextInput`].
///
/// Login, register, and character-name fields are spawned once at startup and
/// only hidden afterwards; calling this on a successful auth dispatch or when
/// entering the game drops leftover focus so those fields cannot keep capturing
/// keybinds.
pub fn unfocus_all(inputs: &mut Query<&mut TextInput>) {
    for mut input in inputs.iter_mut() {
        if input.focused {
            input.focused = false;
        }
    }
}
