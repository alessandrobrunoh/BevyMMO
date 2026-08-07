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

use super::components::{CardExclusivityPolicy, CardWindow, CloseCardButton};

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
    interactions: Query<&Interaction, (Changed<Interaction>, With<CloseCardButton>)>,
    close_buttons: Query<&ChildOf, With<CloseCardButton>>,
    parents: Query<&ChildOf>,
    cards: Query<Entity, With<CardWindow>>,
    mut commands: Commands,
) {
    let any_pressed = interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    if !any_pressed {
        return;
    }

    let card_set: HashSet<Entity> = cards.iter().collect();

    // For every close button whose interaction changed this frame, walk up
    // until we hit a CardWindow ancestor and despawn it.
    for close_parent in close_buttons.iter() {
        let mut current = Some(close_parent.0);
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
