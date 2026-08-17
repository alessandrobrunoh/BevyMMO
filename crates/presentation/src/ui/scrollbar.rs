//! Componente e logica per la Scrollbar e lo ScrollView.

use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    window::PrimaryWindow,
};

use crate::ui::scale::window_to_ui_px;
use crate::ui::theme::UiTheme;

pub struct ScrollbarPlugin;

impl Plugin for ScrollbarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_scroll_max,
                handle_mouse_scroll,
                handle_scrollbar_drag,
                apply_scroll_position,
                update_scrollbar_visuals,
            )
                .chain(),
        );
    }
}

/// Aggiunto al viewport (che clippa il contenuto).
#[derive(Component)]
pub struct ScrollView {
    pub content_entity: Entity,
    pub scrollbar_entity: Option<Entity>,
    pub current_scroll: f32,
    pub max_scroll: f32,
}

/// Aggiunto al contenuto vero e proprio.
#[derive(Component)]
pub struct ScrollContent;

/// Aggiunto al "thumb" della scrollbar per trascinarlo.
#[derive(Component)]
pub struct ScrollbarThumb {
    pub viewport_entity: Entity,
    pub is_dragging: bool,
    pub drag_start_y: f32,
    pub drag_start_scroll: f32,
}

/// Crea una ScrollView. Ritorna l'Entity del wrapper esterno.
pub fn spawn_scroll_view(
    commands: &mut Commands,
    parent: Entity,
    theme: &UiTheme,
    content_builder: impl FnOnce(&mut Commands) -> Entity,
) -> Entity {
    spawn_scroll_view_with_content(commands, parent, theme, content_builder).0
}

/// Crea una ScrollView e ritorna sia il wrapper sia l'entity del contenuto.
///
/// La variante pubblica standard ritorna solo il wrapper; questa serve ai
/// widget che devono aggiungere dinamicamente figli al contenuto scrollabile.
pub fn spawn_scroll_view_with_content(
    commands: &mut Commands,
    parent: Entity,
    theme: &UiTheme,
    content_builder: impl FnOnce(&mut Commands) -> Entity,
) -> (Entity, Entity) {
    let wrapper = commands
        .spawn((Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            ..default()
        },))
        .id();

    commands.entity(parent).add_child(wrapper);

    // Il Viewport che clippa
    let viewport = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip_y(),
                ..default()
            },
            Interaction::default(), // per intercettare l'hover del mouse wheel
        ))
        .id();

    commands.entity(wrapper).add_child(viewport);

    // Il Contenuto
    let content = content_builder(commands);
    commands.entity(content).insert(ScrollContent);
    // Assicuriamoci che il contenuto possa muoversi (Top)
    commands.entity(content).insert(Node {
        top: Val::Px(0.0),
        flex_direction: FlexDirection::Column,
        ..default()
    });
    commands.entity(viewport).add_child(content);

    // Track
    let track = commands
        .spawn((
            Node {
                width: Val::Px(12.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart, // thumb parte da su
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.05, 0.5)),
        ))
        .id();

    // Thumb
    let thumb = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(40.0), // Verrà aggiornato dinamicamente
                ..default()
            },
            BackgroundColor(theme.button_bg),
            ScrollbarThumb {
                viewport_entity: viewport,
                is_dragging: false,
                drag_start_y: 0.0,
                drag_start_scroll: 0.0,
            },
            Interaction::default(),
        ))
        .id();

    commands.entity(track).add_child(thumb);
    commands.entity(wrapper).add_child(track);

    commands.entity(viewport).insert(ScrollView {
        content_entity: content,
        scrollbar_entity: Some(thumb),
        current_scroll: 0.0,
        max_scroll: 0.0,
    });

    (wrapper, content)
}

fn update_scroll_max(
    mut view_q: Query<(&mut ScrollView, &ComputedNode)>,
    content_q: Query<&ComputedNode, With<ScrollContent>>,
) {
    for (mut view, view_node) in view_q.iter_mut() {
        if let Ok(content_node) = content_q.get(view.content_entity) {
            let view_height = view_node.size().y;
            let content_height = content_node.size().y;
            view.max_scroll = (content_height - view_height).max(0.0);
            view.current_scroll = view.current_scroll.clamp(0.0, view.max_scroll);
        }
    }
}

fn handle_mouse_scroll(
    mut mouse_wheel_events: MessageReader<MouseWheel>,
    mut query: Query<(&mut ScrollView, &Interaction)>,
) {
    for event in mouse_wheel_events.read() {
        for (mut scroll_view, interaction) in query.iter_mut() {
            if *interaction == Interaction::Hovered {
                let dy = match event.unit {
                    MouseScrollUnit::Line => event.y * 30.0,
                    MouseScrollUnit::Pixel => event.y,
                };
                scroll_view.current_scroll -= dy;
                scroll_view.current_scroll = scroll_view
                    .current_scroll
                    .clamp(0.0, scroll_view.max_scroll);
            }
        }
    }
}

fn handle_scrollbar_drag(
    mut query: Query<(&Interaction, &mut ScrollbarThumb, &mut BackgroundColor)>,
    mut view_q: Query<(&mut ScrollView, &ComputedNode)>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    theme: Res<UiTheme>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let cursor_y = window.cursor_position().map(|p| p.y).unwrap_or(0.0);

    for (interaction, mut thumb, mut bg) in query.iter_mut() {
        // Avvio drag
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            thumb.is_dragging = true;
            thumb.drag_start_y = cursor_y;
            if let Ok((view, _)) = view_q.get(thumb.viewport_entity) {
                thumb.drag_start_scroll = view.current_scroll;
            }
        }

        // Rilascio drag
        if thumb.is_dragging && mouse_input.just_released(MouseButton::Left) {
            thumb.is_dragging = false;
        }

        // Colori
        if thumb.is_dragging || *interaction == Interaction::Pressed {
            *bg = BackgroundColor(theme.button_pressed_bg);
        } else if *interaction == Interaction::Hovered {
            *bg = BackgroundColor(theme.button_hovered_bg);
        } else {
            *bg = BackgroundColor(theme.button_bg);
        }

        // Calcolo spostamento
        if thumb.is_dragging {
            // `cursor_y` è in px logici della finestra, `size()` in px fisici:
            // vanno riportati nello stesso spazio (UI-logico, quello dei
            // `Val::Px`), altrimenti il rapporto sbaglia del fattore di scala
            // del layout e il trascinamento della barra scorre più lento del
            // mouse.
            let dy = window_to_ui_px(Vec2::new(0.0, cursor_y - thumb.drag_start_y), &ui_scale).y;
            if let Ok((mut view, view_node)) = view_q.get_mut(thumb.viewport_entity) {
                // proporzione per tradurre spostamento del mouse in scroll
                let view_h = view_node.size().y * view_node.inverse_scale_factor();
                // thumb occupa min 20.0 px (es.)
                let max_thumb_travel = view_h - 20.0;
                if max_thumb_travel > 0.0 && view.max_scroll > 0.0 {
                    let scroll_per_px = view.max_scroll / max_thumb_travel;
                    view.current_scroll = thumb.drag_start_scroll + dy * scroll_per_px;
                    view.current_scroll = view.current_scroll.clamp(0.0, view.max_scroll);
                }
            }
        }
    }
}

fn apply_scroll_position(
    mut view_q: Query<&ScrollView>,
    mut content_q: Query<&mut Node, With<ScrollContent>>,
) {
    for view in view_q.iter_mut() {
        if let Ok(mut node) = content_q.get_mut(view.content_entity) {
            node.top = Val::Px(-view.current_scroll);
        }
    }
}

fn update_scrollbar_visuals(
    mut view_q: Query<(&ScrollView, &ComputedNode)>,
    mut thumb_q: Query<(&mut Node, &mut Visibility), With<ScrollbarThumb>>,
) {
    for (view, view_node) in view_q.iter_mut() {
        if let Some(thumb_ent) = view.scrollbar_entity {
            if let Ok((mut thumb_node, mut vis)) = thumb_q.get_mut(thumb_ent) {
                if view.max_scroll <= 0.0 {
                    *vis = Visibility::Hidden;
                } else {
                    *vis = Visibility::Inherited;
                    let view_h = view_node.size().y;

                    // L'altezza del thumb è proporzionale a quanto contenuto è visibile
                    let content_h = view_h + view.max_scroll;
                    let proportion = view_h / content_h.max(1.0);
                    let thumb_h = (view_h * proportion).max(20.0);

                    thumb_node.height = Val::Px(thumb_h);

                    // Posizione
                    let max_thumb_travel = view_h - thumb_h;
                    let scroll_percent = view.current_scroll / view.max_scroll;
                    let thumb_top = scroll_percent * max_thumb_travel;

                    thumb_node.top = Val::Px(thumb_top);
                }
            }
        }
    }
}
