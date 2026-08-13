//! Global Card interaction systems.
//!
//! These run every frame on the (small) set of open cards and handle three
//! concerns:
//!
//! - [`enforce_card_exclusivity`]: when an `Exclusive` card is spawned, despawn
//!   every other non-`Coexist` card. Implements the Policy Object behavior
//!   declared on each [`super::CardWindow`].
//! - [`close_card_on_button`]: clicking a `CloseCardButton` despawns the owning
//!   `CardWindow`.
//! - [`close_card_on_esc`]: pressing `Escape` closes every open card. A LIFO /
//!   focus-aware refinement is left as a follow-up.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use std::collections::HashSet;

use super::components::{
    CardDraggingState, CardExclusivityPolicy, CardHeaderDragHandle, CardWindow, CloseCardButton,
    DraggableCard,
};

/// System to handle dragging draggable Card windows when clicking and dragging their header.
pub fn handle_card_drag(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    header_query: Query<
        (&Interaction, &ChildOf),
        (With<CardHeaderDragHandle>, Changed<Interaction>),
    >,
    card_parents: Query<&ChildOf>,
    mut draggable_cards: Query<
        (Entity, &mut Node, &ComputedNode, &UiGlobalTransform),
        (With<DraggableCard>, Without<CardDraggingState>),
    >,
    mut dragging_query: Query<(Entity, &mut Node, &mut CardDraggingState), With<DraggableCard>>,
    mut commands: Commands,
) {
    let Some(window) = windows.iter().next() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        // Cursor left window, cancel active drags
        for (entity, _, _) in dragging_query.iter() {
            commands.entity(entity).remove::<CardDraggingState>();
        }
        return;
    };

    // 1. Handle drag start on header interaction.
    //
    // Cards are laid out with `CardBuilder`'s viewport-relative centring
    // (`left`/`top` as `Val::Percent` plus a negative half-size `margin`, see
    // `card::builder`), not raw `Val::Px`. Reading `node.left`/`node.top`
    // directly would see `Val::Percent` and silently fall back to `0.0`,
    // making the card jump to a wrong anchor the instant a drag starts. We
    // instead read the card's actual on-screen top-left corner from its
    // computed transform, then rewrite the node to a plain `Val::Px` anchor
    // at that same spot before applying any cursor delta.
    if mouse_button.just_pressed(MouseButton::Left) {
        for (interaction, child) in header_query.iter() {
            if *interaction == Interaction::Pressed {
                // Find root DraggableCard entity by walking up parent tree
                let mut current = Some(child.0);
                while let Some(entity) = current {
                    if let Ok((card_entity, mut node, computed, transform)) =
                        draggable_cards.get_mut(entity)
                    {
                        let top_left = transform.translation - computed.size() * 0.5;

                        node.left = Val::Px(top_left.x);
                        node.top = Val::Px(top_left.y);
                        node.right = Val::Auto;
                        node.bottom = Val::Auto;
                        node.margin = UiRect::all(Val::Px(0.0));

                        commands.entity(card_entity).insert(CardDraggingState {
                            drag_start_cursor: cursor_pos,
                            drag_start_left: top_left.x,
                            drag_start_top: top_left.y,
                        });
                        break;
                    }
                    current = card_parents.get(entity).ok().map(|p| p.0);
                }
            }
        }
    }

    // 2. Handle active dragging
    if mouse_button.pressed(MouseButton::Left) {
        for (_entity, mut node, state) in dragging_query.iter_mut() {
            let delta = cursor_pos - state.drag_start_cursor;
            node.left = Val::Px(state.drag_start_left + delta.x);
            node.top = Val::Px(state.drag_start_top + delta.y);
        }
    } else {
        // Released mouse button, clear drag states
        for (entity, _, _) in dragging_query.iter() {
            commands.entity(entity).remove::<CardDraggingState>();
        }
    }
}

/// When a new `Exclusive` card is added, despawn every other open card whose
/// policy is NOT `Coexist`.
///
/// Runs on `Added<CardWindow>` so it only fires once per card spawn, not every
/// frame. The card set is tiny (a handful at most), so the inner O(n²) is
/// negligible.
pub fn enforce_card_exclusivity(
    new_cards: Query<(Entity, &CardWindow), Added<CardWindow>>,
    all_cards: Query<(Entity, &CardWindow)>,
    mut commands: Commands,
) {
    let mut to_despawn: Vec<Entity> = Vec::new();

    for (new_entity, new_window) in new_cards.iter() {
        if new_window.exclusivity != CardExclusivityPolicy::Exclusive {
            continue;
        }
        for (other_entity, other_window) in all_cards.iter() {
            if other_entity == new_entity {
                continue;
            }
            if other_window.exclusivity == CardExclusivityPolicy::Coexist {
                continue;
            }
            to_despawn.push(other_entity);
        }
    }

    for entity in to_despawn {
        commands.entity(entity).despawn();
    }
}

/// Click on a `CloseCardButton` -> despawn the owning `CardWindow`.
///
/// We walk up the parent chain because the close button is a descendant of the
/// `CardWindow` header. The walk is bounded by the depth of the card tree,
/// which is small by construction.
pub fn close_card_on_button(
    close_buttons: Query<(&Interaction, &ChildOf), (Changed<Interaction>, With<CloseCardButton>)>,
    parents: Query<&ChildOf>,
    cards: Query<Entity, With<CardWindow>>,
    mut commands: Commands,
) {
    // Only the buttons actually pressed this frame close anything. Iterating
    // every close button instead (and gating on "some button was pressed")
    // closed every open card whenever one of them was clicked.
    let pressed_parents: Vec<Entity> = close_buttons
        .iter()
        .filter(|(interaction, _)| **interaction == Interaction::Pressed)
        .map(|(_, close_parent)| close_parent.0)
        .collect();
    if pressed_parents.is_empty() {
        return;
    }

    let card_set: HashSet<Entity> = cards.iter().collect();

    // Walk up from each pressed button until we hit its CardWindow ancestor.
    for start in pressed_parents {
        let mut current = Some(start);
        while let Some(entity) = current {
            if card_set.contains(&entity) {
                commands.entity(entity).despawn();
                break;
            }
            current = parents.get(entity).ok().map(|p| p.0);
        }
    }
}

/// Pressing `Escape` closes every open card.
///
/// This is intentionally global: in the current UI there is no per-card focus
/// tracking. A LIFO / "topmost only" refinement is left as a follow-up when
/// multiple stacked cards become a real UX concern.
pub fn close_card_on_esc(
    keys: Res<ButtonInput<KeyCode>>,
    cards: Query<Entity, With<CardWindow>>,
    mut commands: Commands,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    for entity in cards.iter() {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_card(commands: &mut Commands, exclusivity: CardExclusivityPolicy) -> Entity {
        commands
            .spawn(CardWindow {
                kind: super::super::components::CardKind::Generic,
                exclusivity,
            })
            .id()
    }

    fn run_exclusivity(app: &mut App) {
        app.update();
    }

    #[test]
    fn exclusive_new_card_despawns_other_exclusive_cards() {
        let mut app = App::new();
        app.add_systems(Update, enforce_card_exclusivity);

        let first = {
            let mut commands = app.world_mut().commands();
            spawn_card(&mut commands, CardExclusivityPolicy::Exclusive)
        };
        run_exclusivity(&mut app);
        assert!(app.world().entities().contains(first));

        let _second = {
            let mut commands = app.world_mut().commands();
            spawn_card(&mut commands, CardExclusivityPolicy::Exclusive)
        };
        run_exclusivity(&mut app);

        let world = app.world_mut();
        let remaining: Vec<Entity> = world
            .query_filtered::<Entity, With<CardWindow>>()
            .iter(world)
            .collect();
        assert!(
            !remaining.contains(&first),
            "first exclusive card must be replaced"
        );
        assert_eq!(remaining.len(), 1);
    }

    #[test]
    fn coexist_card_survives_new_exclusive_card() {
        let mut app = App::new();
        app.add_systems(Update, enforce_card_exclusivity);

        let coexist = {
            let mut commands = app.world_mut().commands();
            spawn_card(&mut commands, CardExclusivityPolicy::Coexist)
        };
        run_exclusivity(&mut app);

        let _exclusive = {
            let mut commands = app.world_mut().commands();
            spawn_card(&mut commands, CardExclusivityPolicy::Exclusive)
        };
        run_exclusivity(&mut app);

        assert!(
            app.world().entities().contains(coexist),
            "coexist card survives"
        );
    }

    /// Spawns `card -> header -> close button`, mirroring the real tree built
    /// by [`super::super::builder::CardBuilder`], so the parent walk is exercised.
    fn spawn_card_with_close_button(
        commands: &mut Commands,
        exclusivity: CardExclusivityPolicy,
    ) -> (Entity, Entity) {
        let card = spawn_card(commands, exclusivity);
        let mut button = Entity::PLACEHOLDER;
        commands.entity(card).with_children(|card_root| {
            card_root.spawn(()).with_children(|header| {
                button = header
                    .spawn((
                        CloseCardButton {
                            kind: super::super::components::CardKind::Generic,
                        },
                        Interaction::None,
                    ))
                    .id();
            });
        });
        (card, button)
    }

    #[test]
    fn close_button_only_despawns_its_own_card() {
        let mut app = App::new();
        app.add_systems(Update, close_card_on_button);

        let (first_card, first_button, second_card) = {
            let mut commands = app.world_mut().commands();
            let (first_card, first_button) =
                spawn_card_with_close_button(&mut commands, CardExclusivityPolicy::Coexist);
            let (second_card, _) =
                spawn_card_with_close_button(&mut commands, CardExclusivityPolicy::Coexist);
            (first_card, first_button, second_card)
        };
        app.update();

        *app.world_mut()
            .get_mut::<Interaction>(first_button)
            .unwrap() = Interaction::Pressed;
        app.update();

        assert!(
            !app.world().entities().contains(first_card),
            "the clicked card must close"
        );
        assert!(
            app.world().entities().contains(second_card),
            "an unrelated open card must stay open"
        );
    }

    #[test]
    fn unpressed_close_buttons_do_not_despawn_cards() {
        let mut app = App::new();
        app.add_systems(Update, close_card_on_button);

        let card = {
            let mut commands = app.world_mut().commands();
            spawn_card_with_close_button(&mut commands, CardExclusivityPolicy::Coexist).0
        };
        app.update();
        app.update();

        assert!(app.world().entities().contains(card));
    }

    #[test]
    fn esc_despawns_all_cards() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.add_systems(Update, close_card_on_esc);

        let a = {
            let mut commands = app.world_mut().commands();
            spawn_card(&mut commands, CardExclusivityPolicy::Exclusive)
        };
        let b = {
            let mut commands = app.world_mut().commands();
            spawn_card(&mut commands, CardExclusivityPolicy::Coexist)
        };
        run_exclusivity(&mut app);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        run_exclusivity(&mut app);

        assert!(!app.world().entities().contains(a));
        assert!(!app.world().entities().contains(b));
    }
}
