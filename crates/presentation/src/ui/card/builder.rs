//! `CardBuilder` — Builder pattern for spawning a standard Card.
//!
//! Building a Bevy UI `Node` tree by hand is verbose and easy to get
//! inconsistently styled across panels. The builder centralizes the header /
//! footer / padding / theme / exclusivity so every future panel passes through
//! a single call site.

use std::borrow::Cow;

use bevy::prelude::*;

use super::components::{
    CardBody, CardExclusivityPolicy, CardFooter, CardHeader, CardHeaderDragHandle, CardKind,
    CardPositioning, CardWindow, CloseCardButton, DraggableCard,
};
use crate::ui::theme::UiTheme;

/// Default card geometry. Callers override via [`CardBuilder::width`] /
/// [`CardBuilder::height`].
pub const DEFAULT_CARD_WIDTH: f32 = 520.0;
pub const DEFAULT_CARD_HEIGHT: f32 = 360.0;
const HEADER_HEIGHT: f32 = 44.0;
const INNER_PADDING: f32 = 14.0;
const HEADER_BOTTOM_GAP: f32 = 12.0;
/// Gap between a `CardPositioning::Right` card and the right edge of the viewport.
const RIGHT_EDGE_GAP: f32 = 40.0;

/// Layout variant for the close button inside the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CardLayout {
    /// Header shows only the title; no close button.
    #[default]
    NoClose,
    /// Header shows the title on the left and a `Close` button on the right.
    WithClose,
}

/// Closure that spawns the body or footer children of a card.
type CardContentSpawner<'a> = Box<dyn FnOnce(&mut ChildSpawnerCommands) + 'a>;

/// Builder for a standard Card.
pub struct CardBuilder<'a> {
    kind: CardKind,
    title: Cow<'a, str>,
    width: Val,
    height: Val,
    layout: CardLayout,
    exclusivity: CardExclusivityPolicy,
    positioning: CardPositioning,
    draggable: bool,
    body: CardContentSpawner<'a>,
    footer: Option<CardContentSpawner<'a>>,
}

impl<'a> CardBuilder<'a> {
    /// Starts a new card. Title is shown in the header.
    pub fn new(kind: CardKind, title: impl Into<Cow<'a, str>>) -> Self {
        Self {
            kind,
            title: title.into(),
            width: Val::Px(DEFAULT_CARD_WIDTH),
            height: Val::Px(DEFAULT_CARD_HEIGHT),
            layout: CardLayout::NoClose,
            exclusivity: CardExclusivityPolicy::default(),
            positioning: CardPositioning::Center,
            draggable: false,
            body: Box::new(|_| {}),
            footer: None,
        }
    }

    /// Overrides the card width.
    pub fn width(mut self, width: Val) -> Self {
        self.width = width;
        self
    }

    /// Overrides the card height.
    pub fn height(mut self, height: Val) -> Self {
        self.height = height;
        self
    }

    /// Sets the card positioning (Center or Right).
    pub fn positioning(mut self, positioning: CardPositioning) -> Self {
        self.positioning = positioning;
        self
    }

    /// Enables dragging the card window around by holding its header.
    pub fn draggable(mut self) -> Self {
        self.draggable = true;
        self
    }

    /// Adds a `Close` button to the header (sets layout to [`CardLayout::WithClose`]).
    pub fn closeable(mut self) -> Self {
        self.layout = CardLayout::WithClose;
        self
    }

    /// Marks this card as `Exclusive` (closes other non-`Coexist` cards on open).
    pub fn exclusive(mut self) -> Self {
        self.exclusivity = CardExclusivityPolicy::Exclusive;
        self
    }

    /// Marks this card as `Coexist` (can stay open alongside other cards).
    pub fn coexist(mut self) -> Self {
        self.exclusivity = CardExclusivityPolicy::Coexist;
        self
    }

    /// Supplies the body content.
    pub fn with_body<F>(mut self, body: F) -> Self
    where
        F: FnOnce(&mut ChildSpawnerCommands) + 'a,
    {
        self.body = Box::new(body);
        self
    }

    /// Supplies an optional footer.
    pub fn with_footer<F>(mut self, footer: F) -> Self
    where
        F: FnOnce(&mut ChildSpawnerCommands) + 'a,
    {
        self.footer = Some(Box::new(footer));
        self
    }

    /// Spawns the card into the world and returns the root `CardWindow` entity.
    pub fn spawn(self, commands: &mut Commands<'_, '_>, theme: &UiTheme) -> Entity {
        let Self {
            kind,
            title,
            width,
            height,
            layout,
            exclusivity,
            positioning,
            draggable,
            body,
            footer,
        } = self;

        let header_style = HeaderStyle::from_theme(theme);

        // Cards are placed relative to the viewport, never to a fixed
        // resolution: a 50% inset plus a negative half-size margin, the same
        // pattern `ui::spellbook` uses. Centring against a hardcoded 1920x1080
        // put every card partly or fully off-screen at the default 800x600
        // window (`bins/game/src/main.rs`).
        //
        // Sizes whose extent is not known at build time (anything but
        // `Val::Px`) fall back to `margin: auto` centring, which does not need
        // the size.
        let (top, bottom, margin_top) = match height {
            Val::Px(h) => (Val::Percent(50.0), Val::Auto, Val::Px(-h * 0.5)),
            _ => (Val::Px(0.0), Val::Px(0.0), Val::Auto),
        };
        let (left, right, margin_left) = match positioning {
            CardPositioning::Center => match width {
                Val::Px(w) => (Val::Percent(50.0), Val::Auto, Val::Px(-w * 0.5)),
                _ => (Val::Px(0.0), Val::Px(0.0), Val::Auto),
            },
            CardPositioning::Right => (Val::Auto, Val::Px(RIGHT_EDGE_GAP), Val::Auto),
        };
        let margin = UiRect {
            left: margin_left,
            right: Val::Auto,
            top: margin_top,
            bottom: Val::Auto,
        };

        let mut card_entity = commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                width,
                height,
                left,
                right,
                top,
                bottom,
                margin,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(INNER_PADDING)),
                row_gap: Val::Px(HEADER_BOTTOM_GAP),
                border: UiRect::all(Val::Px(1.5)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(theme.panel_bg),
            BorderColor {
                top: Color::srgba(0.35, 0.38, 0.45, 0.6),
                right: Color::srgba(0.35, 0.38, 0.45, 0.6),
                bottom: Color::srgba(0.35, 0.38, 0.45, 0.6),
                left: Color::srgba(0.35, 0.38, 0.45, 0.6),
            },
            CardWindow { kind, exclusivity },
        ));

        if draggable {
            card_entity.insert(DraggableCard);
        }

        card_entity
            .with_children(|card_root| {
                spawn_header(card_root, kind, layout, &title, &header_style, draggable);

                card_root
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        CardBody,
                    ))
                    .with_children(body);

                if let Some(footer_fn) = footer {
                    card_root
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(8.0),
                                padding: UiRect::top(Val::Px(8.0)),
                                border: UiRect::top(Val::Px(1.0)),
                                ..default()
                            },
                            BorderColor {
                                top: Color::srgba(1.0, 1.0, 1.0, 0.1),
                                right: Color::NONE,
                                bottom: Color::NONE,
                                left: Color::NONE,
                            },
                            CardFooter,
                        ))
                        .with_children(footer_fn);
                }
            })
            .id()
    }
}

fn spawn_header(
    parent: &mut ChildSpawnerCommands,
    kind: CardKind,
    layout: CardLayout,
    title: &str,
    style: &HeaderStyle,
    draggable: bool,
) {
    let mut header_cmd = parent.spawn((
        Button,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(HEADER_HEIGHT),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
        CardHeader,
    ));

    if draggable {
        header_cmd.insert(CardHeaderDragHandle);
    }

    header_cmd.with_children(|header| {
        let title_text = title.to_string();

        header
            .spawn((Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },))
            .with_children(|title_node| {
                if draggable {
                    title_node.spawn((
                        Text::new("≡".to_string()),
                        TextFont {
                            font_size: FontSize::Px(style.title_font_size * 1.1),
                            ..default()
                        },
                        TextColor(Color::srgba(0.7, 0.75, 0.85, 0.8)),
                    ));
                }

                title_node.spawn((
                    Text::new(title_text),
                    TextFont {
                        font_size: FontSize::Px(style.title_font_size),
                        ..default()
                    },
                    TextColor(style.text_color),
                ));
            });

        if layout == CardLayout::WithClose {
            spawn_close_button(header, kind, style);
        }
    });
}

fn spawn_close_button(header: &mut ChildSpawnerCommands, kind: CardKind, style: &HeaderStyle) {
    header
        .spawn((
            Button,
            Node {
                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(style.button_bg),
            CloseCardButton { kind },
        ))
        .with_children(|button| {
            button.spawn((
                Text::new("Close".to_string()),
                TextFont {
                    font_size: FontSize::Px(style.button_font_size),
                    ..default()
                },
                TextColor(style.button_text_color),
            ));
        });
}

/// Bundles the theme values used by the header so helper functions stay
/// under clippy's argument-count threshold.
struct HeaderStyle {
    title_font_size: f32,
    button_font_size: f32,
    text_color: Color,
    button_bg: Color,
    button_text_color: Color,
}

impl HeaderStyle {
    fn from_theme(theme: &UiTheme) -> Self {
        Self {
            title_font_size: theme.title_font_size,
            button_font_size: theme.button_font_size,
            text_color: theme.text_color,
            button_bg: theme.button_bg,
            button_text_color: theme.button_text_color,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_theme() -> UiTheme {
        UiTheme::default()
    }

    #[test]
    fn spawn_creates_one_card_window_with_header_and_body() {
        let mut app = App::new();
        let theme = test_theme();
        let mut commands = app.world_mut().commands();

        let card_entity = CardBuilder::new(CardKind::Generic, "Test")
            .with_body(|body| {
                body.spawn(Text::new("hello"));
            })
            .spawn(&mut commands, &theme);

        app.update();

        let world = app.world();
        let window = world
            .get::<CardWindow>(card_entity)
            .expect("card root has CardWindow");
        assert_eq!(window.kind, CardKind::Generic);
        assert_eq!(window.exclusivity, CardExclusivityPolicy::Exclusive);

        let world = app.world_mut();
        let mut headers = world.query::<&CardHeader>();
        assert_eq!(headers.iter(world).count(), 1);

        let world = app.world_mut();
        let mut bodies = world.query::<&CardBody>();
        assert_eq!(bodies.iter(world).count(), 1);

        let world = app.world_mut();
        let mut footers = world.query::<&CardFooter>();
        assert_eq!(footers.iter(world).count(), 0);
    }

    #[test]
    fn closeable_adds_close_button() {
        let mut app = App::new();
        let theme = test_theme();
        let mut commands = app.world_mut().commands();

        CardBuilder::new(CardKind::Generic, "Test")
            .closeable()
            .spawn(&mut commands, &theme);

        app.update();

        let world = app.world_mut();
        let mut close_buttons = world.query::<&CloseCardButton>();
        assert_eq!(close_buttons.iter(world).count(), 1);
    }

    /// Regression: centring used to be computed against a hardcoded 1920x1080,
    /// so at the default 800x600 window every card spawned off-screen. The
    /// placement must be expressed in viewport-relative terms instead.
    #[test]
    fn centered_card_is_positioned_relative_to_the_viewport() {
        let mut app = App::new();
        let theme = test_theme();
        let mut commands = app.world_mut().commands();

        let entity = CardBuilder::new(CardKind::Generic, "Test")
            .width(Val::Px(400.0))
            .height(Val::Px(300.0))
            .spawn(&mut commands, &theme);

        app.update();

        let node = app.world().get::<Node>(entity).expect("card node");
        assert_eq!(node.left, Val::Percent(50.0));
        assert_eq!(node.top, Val::Percent(50.0));
        assert_eq!(node.margin.left, Val::Px(-200.0));
        assert_eq!(node.margin.top, Val::Px(-150.0));
    }

    #[test]
    fn right_positioned_card_anchors_to_the_right_edge() {
        let mut app = App::new();
        let theme = test_theme();
        let mut commands = app.world_mut().commands();

        let entity = CardBuilder::new(CardKind::Inventory, "Inventory")
            .width(Val::Px(400.0))
            .height(Val::Px(300.0))
            .positioning(CardPositioning::Right)
            .spawn(&mut commands, &theme);

        app.update();

        let node = app.world().get::<Node>(entity).expect("card node");
        assert_eq!(node.right, Val::Px(RIGHT_EDGE_GAP));
        assert_eq!(node.left, Val::Auto);
        // Vertically centred like any other card.
        assert_eq!(node.top, Val::Percent(50.0));
        assert_eq!(node.margin.top, Val::Px(-150.0));
    }

    #[test]
    fn coexist_flag_is_preserved_on_card_window() {
        let mut app = App::new();
        let theme = test_theme();
        let mut commands = app.world_mut().commands();

        let entity = CardBuilder::new(CardKind::ItemDetail, "Detail")
            .coexist()
            .spawn(&mut commands, &theme);

        app.update();

        let world = app.world();
        let window = world.get::<CardWindow>(entity).expect("card window");
        assert_eq!(window.exclusivity, CardExclusivityPolicy::Coexist);
    }
}
