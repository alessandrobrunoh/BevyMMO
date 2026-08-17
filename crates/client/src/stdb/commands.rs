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
//! # Two different failures
//!
//! Every call here goes through the generated `*_then` form, which takes a
//! callback and runs it when the reducer's own result comes back. That splits
//! the two things that can go wrong:
//!
//! - the returned `Result` is *transport*: the request could not be handed to
//!   the SDK at all. The caller sees it immediately and logs it.
//! - the callback carries the module's `Result<(), String>` — "inventory is
//!   full", "target is out of range", "that name is taken". It arrives later, on
//!   the SDK's thread, so it is pushed onto the same channel row changes use and
//!   surfaces as a [`crate::server_feed::ServerNotice`].
//!
//! The plain fire-and-forget forms reported only the first kind, which is why
//! every carefully worded refusal in the module used to vanish.

use bevy::prelude::Vec3;
use bevymmo_domain::abilities::AbilitySlot;
use bevymmo_domain::items::EquipSlot;
use bevymmo_domain::spells::components::HotbarSlot;

use super::module_bindings::armor_cast_reducer::armor_cast as armor_cast_reducer;
use super::module_bindings::send_chat_message_reducer::send_chat_message as send_chat_message_reducer;
use super::module_bindings::claim_npc_item_reducer::claim_npc_item as claim_npc_item_reducer;
use super::module_bindings::eidolon_cast_reducer::eidolon_cast as eidolon_cast_reducer;
use super::module_bindings::destroy_item_reducer::destroy_item as destroy_item_reducer;
use super::module_bindings::equip_item_reducer::equip_item as equip_item_reducer;
use super::module_bindings::move_item_reducer::move_item as move_item_reducer;
use super::module_bindings::party_accept_reducer::party_accept as party_accept_reducer;
use super::module_bindings::party_decline_reducer::party_decline as party_decline_reducer;
use super::module_bindings::party_invite_reducer::party_invite as party_invite_reducer;
use super::module_bindings::party_join_reducer::party_join as party_join_reducer;
use super::module_bindings::party_leave_reducer::party_leave as party_leave_reducer;
use super::module_bindings::release_cast_reducer::release_cast as release_cast_reducer;
use super::module_bindings::respawn_reducer::respawn as respawn_reducer;
use super::module_bindings::set_ability_selection_reducer::set_ability_selection as set_ability_selection_reducer;
use super::module_bindings::set_hotbar_spell_reducer::set_hotbar_spell as set_hotbar_spell_reducer;
use super::module_bindings::set_inscription_reducer::set_inscription as set_inscription_reducer;
use super::module_bindings::unequip_item_reducer::unequip_item as unequip_item_reducer;
use super::module_bindings::Vec3Row;
use super::plugin::StdbConnection;

/// Whether the *request* reached the SDK. The server's own answer arrives
/// later, through the rejection callback.
type Sent = Result<(), spacetimedb_sdk::Error>;

fn to_row(v: Vec3) -> Vec3Row {
    Vec3Row {
        x: v.x,
        y: v.y,
        z: v.z,
    }
}

/// Claims a catalogue item from a nearby NPC vendor.
pub fn claim_npc_item(conn: &StdbConnection, npc_entity_id: u64, item_id: String) -> Sent {
    conn.reducers().claim_npc_item_then(
        npc_entity_id,
        item_id,
        conn.report_rejection("could not claim that item"),
    )
}

/// Permanently destroys an item instance from the inventory.
pub fn destroy_item(conn: &StdbConnection, instance_id: u64) -> Sent {
    conn.reducers()
        .destroy_item_then(instance_id, conn.report_rejection("could not destroy that item"))
}

/// Moves an inventory item into the equipment slot its definition allows.
pub fn equip_item(conn: &StdbConnection, slot_index: u8) -> Sent {
    conn.reducers()
        .equip_item_then(slot_index, conn.report_rejection("could not equip"))
}

/// Takes an equipped item off and puts it in the first free inventory slot.
pub fn unequip_item(conn: &StdbConnection, slot: EquipSlot) -> Sent {
    conn.reducers().unequip_item_then(
        slot.label().to_string(),
        conn.report_rejection("could not unequip"),
    )
}

/// Swaps two inventory slots.
pub fn move_item(conn: &StdbConnection, from: u8, to: u8) -> Sent {
    conn.reducers()
        .move_item_then(from, to, conn.report_rejection("could not move that item"))
}

/// Binds a spell to a hotbar key, or clears it with `None`.
pub fn set_hotbar_spell(conn: &StdbConnection, slot: HotbarSlot, spell_id: Option<String>) -> Sent {
    conn.reducers().set_hotbar_spell_then(
        hotbar_label(slot).to_string(),
        spell_id,
        conn.report_rejection("could not bind that spell"),
    )
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
    conn.reducers().set_inscription_then(
        ability_label(slot).to_string(),
        essence,
        modifiers,
        ancient_word,
        conn.report_rejection("could not write that inscription"),
    )
}

/// Chooses which of the weapon's offered abilities is active in a slot.
pub fn set_ability_selection(conn: &StdbConnection, slot: AbilitySlot, ability_id: String) -> Sent {
    conn.reducers().set_ability_selection_then(
        ability_label(slot).to_string(),
        ability_id,
        conn.report_rejection("could not choose that ability"),
    )
}

/// Ends a channelled cast. Naming the spell stops a stale release from
/// cancelling a cast that started after it.
pub fn release_cast(conn: &StdbConnection, spell_id: String) -> Sent {
    conn.reducers()
        .release_cast_then(spell_id, conn.report_rejection("could not end that cast"))
}

/// Casts the weapon's Eidolon gesture bound to an ability slot.
pub fn eidolon_cast(
    conn: &StdbConnection,
    slot: AbilitySlot,
    target_entity: Option<u64>,
    target_position: Option<Vec3>,
) -> Sent {
    conn.reducers().eidolon_cast_then(
        ability_label(slot).to_string(),
        target_entity,
        target_position.map(to_row),
        conn.report_rejection("could not cast that gesture"),
    )
}

/// Casts the first active ability supplied by an equipped armor item.
///
/// The server resolves the Armor inscription and cast mode; the client only
/// identifies the equipment slot and target.
pub fn armor_cast(
    conn: &StdbConnection,
    slot: EquipSlot,
    ability_slot: AbilitySlot,
    target_entity: Option<u64>,
    target_position: Option<Vec3>,
) -> Sent {
    conn.reducers().armor_cast_then(
        slot.label().to_ascii_lowercase(),
        ability_label(ability_slot).to_string(),
        target_entity,
        target_position.map(to_row),
        conn.report_rejection("could not cast that armor ability"),
    )
}

/// Sends a message to the global server chat.
pub fn send_chat_message(conn: &StdbConnection, text: String) -> Sent {
    conn.reducers()
        .send_chat_message_then(text, conn.report_rejection("could not send chat message"))
}

/// `/party invite <name>` — invites `target_name`, implicitly creating a
/// party with the sender as leader if they are not already in one.
pub fn party_invite(conn: &StdbConnection, target_name: String) -> Sent {
    conn.reducers()
        .party_invite_then(target_name, conn.report_rejection("could not invite"))
}

/// `/party join <name>` — asks to join `leader_name`'s party.
pub fn party_join(conn: &StdbConnection, leader_name: String) -> Sent {
    conn.reducers()
        .party_join_then(leader_name, conn.report_rejection("could not ask to join"))
}

/// `/party accept <name>` — accepts the pending request between the caller
/// and `name`, whichever direction it runs.
pub fn party_accept(conn: &StdbConnection, name: String) -> Sent {
    conn.reducers()
        .party_accept_then(name, conn.report_rejection("could not accept"))
}

/// `/party decline <name>` — declines the pending request between the caller
/// and `name`, whichever direction it runs.
pub fn party_decline(conn: &StdbConnection, name: String) -> Sent {
    conn.reducers()
        .party_decline_then(name, conn.report_rejection("could not decline"))
}

/// `/party leave` — leaves the caller's current party.
pub fn party_leave(conn: &StdbConnection) -> Sent {
    conn.reducers()
        .party_leave_then(conn.report_rejection("could not leave the party"))
}

/// Brings a dead character back at its spawn point.
pub fn respawn(conn: &StdbConnection) -> Sent {
    conn.reducers()
        .respawn_then(conn.report_rejection("could not respawn"))
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
