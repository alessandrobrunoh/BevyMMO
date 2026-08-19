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
/// `input_gold.png` / `input_password_gold.png` (divider measured on these).
const INPUT_NATIVE_HEIGHT: f32 = 146.0;
/// Divider ends at x=160 (user) / x=162 (password). +8 so the icon stays in
/// the cap; leftover pixels in the center strip stretch under the text.
const INPUT_SLICE_LEFT: f32 = 170.0;
/// Right diamonds; password eye begins at x≈559 of 689.
const INPUT_SLICE_RIGHT: f32 = 142.0;
const INPUT_SLICE_Y: f32 = 30.0;
/// Larger than the rendered left cap (`SLICE_LEFT * HEIGHT / NATIVE` ≈ 67).
/// 72 sat on the icon; 116 starts glyphs in the empty field.
const INPUT_PAD_LEFT: f32 = 116.0;
const INPUT_PAD_RIGHT: f32 = 28.0;
/// Clears the baked eye (`SLICE_RIGHT * HEIGHT / NATIVE + 12` ≈ 68).
const PASSWORD_PAD_RIGHT: f32 = 68.0;

const _: () = {
    assert!(INPUT_PAD_LEFT > 90.0);
    assert!(PASSWORD_PAD_RIGHT > INPUT_PAD_RIGHT);
    const RENDERED_CAP: f32 = INPUT_SLICE_LEFT * (INPUT_HEIGHT / INPUT_NATIVE_HEIGHT);
    assert!(INPUT_PAD_LEFT > RENDERED_CAP + 12.0);
};

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
    /// Clip/scroll viewport that holds [`value_text`].
    pub(crate) viewport: Entity,
}

/// Marker on the clip viewport inside a [`TextInput`].
#[derive(Component)]
pub struct TextInputViewport;

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
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            TextInputValueText,
            Pickable::IGNORE,
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

    // Inset the value so glyphs start past the 9-sliced icon cap, not on it.
    // `scroll_x` + ScrollPosition keep a long value inside the field the way
    // a normal single-line input does (caret stays at the visible end).
    let content = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::new(
                    Val::Px(INPUT_PAD_LEFT),
                    Val::Px(pad_right),
                    Val::Px(0.0),
                    Val::Px(0.0),
                ),
                align_items: AlignItems::Center,
                overflow: Overflow::scroll_x(),
                ..default()
            },
            ScrollPosition::default(),
            TextInputViewport,
            Pickable::IGNORE,
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
                viewport: content,
            },
        ))
        .id();

    commands.entity(wrapper).add_child(input_entity);
    commands.entity(input_entity).add_child(content);
    commands.entity(content).add_child(value_text);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>();
        app.init_resource::<UiTheme>();
        app
    }

    fn spawn_field(obscured: bool) -> (App, Entity) {
        let mut app = test_app();
        let theme = UiTheme::default();
        let parent = app.world_mut().spawn(Node::default()).id();
        let asset_server = app.world().resource::<AssetServer>().clone();
        let entity = {
            let mut commands = app.world_mut().commands();
            if obscured {
                spawn_password_input(&mut commands, parent, "Password", 32, &theme, &asset_server)
            } else {
                spawn_text_input(&mut commands, parent, "Email", 32, &theme, &asset_server)
            }
        };
        app.world_mut().flush();
        (app, entity)
    }

    fn content_node(app: &App, input: Entity) -> Entity {
        let children = app
            .world()
            .get::<Children>(input)
            .expect("sliced bar has an inner content node");
        children[0]
    }

    #[test]
    fn input_pad_left_clears_baked_icon() {
        let (app, input) = spawn_field(false);
        let content = app.world().get::<Node>(content_node(&app, input)).unwrap();
        let Val::Px(left) = content.padding.left else {
            panic!("left pad must be Px, got {:?}", content.padding.left);
        };
        assert!(
            left > 90.0,
            "left pad must clear the baked user/lock icon; 72px sits on it after 9-slice scale"
        );
    }

    #[test]
    fn spawn_text_input_nests_value_in_padded_content() {
        let (app, input) = spawn_field(false);

        let node = app.world().get::<Node>(input).expect("Node");
        assert_eq!(node.height, Val::Px(INPUT_HEIGHT));
        assert_eq!(node.width, Val::Percent(100.0));
        assert_eq!(node.max_width, Val::Px(INPUT_MAX_WIDTH));
        assert_eq!(node.padding.left, Val::Px(0.0));

        let image = app.world().get::<ImageNode>(input).expect("ImageNode");
        assert!(
            matches!(image.image_mode, NodeImageMode::Sliced(_)),
            "input bars must 9-slice so the icon cap does not stretch under the text"
        );
        assert!(app.world().get::<TextInputImages>(input).is_some());
        assert!(app.world().get::<Button>(input).is_some());

        let content = content_node(&app, input);
        let inner = app.world().get::<Node>(content).expect("content Node");
        assert_eq!(inner.width, Val::Percent(100.0));
        assert_eq!(inner.height, Val::Percent(100.0));
        assert_eq!(inner.padding.left, Val::Px(INPUT_PAD_LEFT));
        assert_eq!(inner.padding.right, Val::Px(INPUT_PAD_RIGHT));
        assert_eq!(inner.align_items, AlignItems::Center);
        assert_eq!(inner.overflow, Overflow::scroll_x());
        assert!(app.world().get::<ScrollPosition>(content).is_some());

        let value_text = app.world().get::<TextInput>(input).unwrap().value_text;
        let content_children = app.world().get::<Children>(content).expect("text child");
        assert_eq!(content_children[0], value_text);

        let layout = app
            .world()
            .get::<TextLayout>(value_text)
            .expect("TextLayout");
        assert_eq!(layout.linebreak, LineBreak::NoWrap);
    }

    #[test]
    fn spawn_password_input_pads_past_the_eye() {
        let (app, input) = spawn_field(true);
        let content = content_node(&app, input);
        let inner = app.world().get::<Node>(content).expect("content Node");
        assert_eq!(inner.padding.left, Val::Px(INPUT_PAD_LEFT));
        assert_eq!(inner.padding.right, Val::Px(PASSWORD_PAD_RIGHT));
        assert!(app.world().get::<TextInput>(input).unwrap().obscured);
    }

    #[test]
    fn value_text_does_not_shrink_so_it_can_scroll() {
        let (app, input) = spawn_field(false);
        let value_text = app.world().get::<TextInput>(input).unwrap().value_text;
        let node = app.world().get::<Node>(value_text).expect("value node");
        assert_eq!(node.flex_shrink, 0.0);
    }
}
