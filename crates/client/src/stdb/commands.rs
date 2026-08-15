//! Typed wrappers around the module's reducers.
//!
//! The UI used to send lightyear messages: `sender.send::<Channel2>(EquipItemCommand
//! { slot_index })`. It now calls a reducer, and this module is the seam — so
//! that `bevymmo_presentation` never imports the generated bindings, and so the
//! reducers' stringly-typed parameters are built in exactly one place.
//!
//! The module parses slot names case-insensitively from the domain's own
//! labels; passing the enum here and converting once means a rename in
//! `EquipSlot` cannot silently start sending an unknown slot name.
//!
//! Every call returns `Result`, and every failure is a real one worth showing:
//! the server rejects an equip that fails its requirements, a cast that is on
//! cooldown, a hotbar spell the character cannot use. The old code could not
//! report any of that — a lightyear send was fire-and-forget.

use bevymmo_domain::abilities::AbilitySlot;
use bevymmo_domain::items::EquipSlot;
use bevymmo_domain::spells::components::HotbarSlot;
use bevy::prelude::Vec3;

use super::module_bindings::cast_spell_reducer::cast_spell as cast_spell_reducer;
use super::module_bindings::eidolon_cast_reducer::eidolon_cast as eidolon_cast_reducer;
use super::module_bindings::equip_item_reducer::equip_item as equip_item_reducer;
use super::module_bindings::move_item_reducer::move_item as move_item_reducer;
use super::module_bindings::release_cast_reducer::release_cast as release_cast_reducer;
use super::module_bindings::respawn_reducer::respawn as respawn_reducer;
use super::module_bindings::set_ability_selection_reducer::set_ability_selection as set_ability_selection_reducer;
use super::module_bindings::set_hotbar_spell_reducer::set_hotbar_spell as set_hotbar_spell_reducer;
use super::module_bindings::set_inscription_reducer::set_inscription as set_inscription_reducer;
use super::module_bindings::unequip_item_reducer::unequip_item as unequip_item_reducer;
use super::module_bindings::Vec3Row;
use super::plugin::StdbConnection;

type Sent = Result<(), spacetimedb_sdk::Error>;

fn to_row(v: Vec3) -> Vec3Row {
    Vec3Row {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

/// Moves an inventory item into the equipment slot its definition allows.
pub fn equip_item(conn: &StdbConnection, slot_index: u8) -> Sent {
    conn.reducers().equip_item(slot_index)
}

/// Takes an equipped item off and puts it in the first free inventory slot.
pub fn unequip_item(conn: &StdbConnection, slot: EquipSlot) -> Sent {
    conn.reducers().unequip_item(slot.label().to_string())
}

/// Swaps two inventory slots.
pub fn move_item(conn: &StdbConnection, from: u8, to: u8) -> Sent {
    conn.reducers().move_item(from, to)
}

/// Binds a spell to a hotbar key, or clears it with `None`.
pub fn set_hotbar_spell(conn: &StdbConnection, slot: HotbarSlot, spell_id: Option<String>) -> Sent {
    conn.reducers()
        .set_hotbar_spell(hotbar_label(slot).to_string(), spell_id)
}

/// Rewrites one ability slot's inscription: an essence, some modifiers, and an
/// optional ancient word.
pub fn set_inscription(
    conn: &StdbConnection,
    slot: AbilitySlot,
    essence: Option<String>,
    modifiers: Vec<String>,
    ancient_word: Option<String>,
) -> Sent {
    conn.reducers().set_inscription(
        ability_label(slot).to_string(),
        essence,
        modifiers,
        ancient_word,
    )
}

/// Chooses which of the weapon's offered abilities is active in a slot.
pub fn set_ability_selection(conn: &StdbConnection, slot: AbilitySlot, ability_id: String) -> Sent {
    conn.reducers()
        .set_ability_selection(ability_label(slot).to_string(), ability_id)
}

/// Casts a hotbar spell at an entity, a point, or neither (a self-cast).
pub fn cast_spell(
    conn: &StdbConnection,
    spell_id: String,
    target_entity: Option<u64>,
    target_position: Option<Vec3>,
) -> Sent {
    conn.reducers()
        .cast_spell(spell_id, target_entity, target_position.map(to_row))
}

/// Ends a channelled cast. Naming the spell stops a stale release from
/// cancelling a cast that started after it.
pub fn release_cast(conn: &StdbConnection, spell_id: String) -> Sent {
    conn.reducers().release_cast(spell_id)
}

/// Casts the weapon's Eidolon gesture bound to an ability slot.
pub fn eidolon_cast(
    conn: &StdbConnection,
    slot: AbilitySlot,
    target_entity: Option<u64>,
    target_position: Option<Vec3>,
) -> Sent {
    conn.reducers().eidolon_cast(
        ability_label(slot).to_string(),
        target_entity,
        target_position.map(to_row),
    )
}

/// Brings a dead character back at its spawn point.
pub fn respawn(conn: &StdbConnection) -> Sent {
    conn.reducers().respawn()
}

fn hotbar_label(slot: HotbarSlot) -> &'static str {
    match slot {
        HotbarSlot::Q => "q",
        HotbarSlot::W => "w",
        HotbarSlot::E => "e",
    }
}

fn ability_label(slot: AbilitySlot) -> &'static str {
    match slot {
        AbilitySlot::Primary => "primary",
        AbilitySlot::Secondary => "secondary",
        AbilitySlot::Ultimate => "ultimate",
    }
}
