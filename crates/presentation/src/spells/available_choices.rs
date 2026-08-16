//! Client-side mirror of the server's `AvailableSpellChoices` recomputation.
//!
//! `AvailableSpellChoices` is not replicated (see its doc comment in
//! `bevymmo_shared::items::available_spells`): `Equipment` already is, and
//! `ItemRegistry` is populated identically on both sides at `Startup`, so the
//! client derives the exact same value locally instead of waiting on a
//! network round trip. This system only maintains the read side for the
//! locally controlled player — it never mutates `SpellHotbar` itself, that
//! stays server-authoritative (see `bevymmo_module::reducers::items`); the
//! client just needs the pool to render the legacy spell-selection UI.
//!
//! Without this the spell selector's own query — `&AvailableSpellChoices` as
//! a required component — never matches, so the window can never open: the
//! component simply never exists on the client side.
//!
//! Runs unconditionally every frame rather than on `Changed<Equipment>`:
//! there is only ever one controlled entity, so the cost is a handful of
//! hashmap lookups, and avoiding the change-detection gate sidesteps the
//! one-frame command-flush lag of inserting the component on first sight of
//! a freshly spawned/replicated player entity.

use bevy::prelude::*;
use bevymmo_shared::entity::LocalPlayer;

use bevymmo_shared::items::components::Equipment;
use bevymmo_shared::items::registry::ItemRegistry;
use bevymmo_shared::items::{compute_available_choices, AvailableSpellChoices};

pub fn sync_available_spell_choices(
    mut commands: Commands,
    mut players: Query<(Entity, &Equipment, Option<&mut AvailableSpellChoices>), With<LocalPlayer>>,
    registry: Res<ItemRegistry>,
) {
    for (entity, equipment, choices) in &mut players {
        let recomputed = compute_available_choices(equipment, &registry);
        match choices {
            Some(mut choices) => {
                if *choices != recomputed {
                    *choices = recomputed;
                }
            }
            None => {
                commands.entity(entity).insert(recomputed);
            }
        }
    }
}
