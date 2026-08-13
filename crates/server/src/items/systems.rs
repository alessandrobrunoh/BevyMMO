//! Server handlers for inventory/equipment client commands.
//!
//! Each handler reads a client command from the connected link, resolves the
//! owning player, and delegates the mutation to a pure helper function that is
//! unit-tested in isolation. All validation happens server-side: the client is
//! never trusted as an authoritative source of ECS state.

use bevy::prelude::*;
use lightyear::connection::client::Connected;
use lightyear::prelude::*;

use bevymmo_shared::entity::player::components::Player;
use bevymmo_shared::items::components::{EquipSlot, Equipment, Inventory, INVENTORY_CAPACITY};
use bevymmo_shared::items::events::{EquipItemCommand, MoveItemCommand, UnequipItemCommand};
use bevymmo_shared::items::registry::ItemRegistry;
use bevymmo_shared::network::protocol::PlayerId;

use crate::network::server::{DbPlayerId, Joined};
use crate::persistence::PersistenceRuntime;
use crate::persistence::PlayerStore;

/// Equips the item in `slot_index` into its declared equipment slot.
///
/// Validation order (all failures are logged and skipped, never trusted from
/// the client):
/// 1. `slot_index` must be a valid inventory index;
/// 2. the slot must hold an item;
/// 3. the item must exist in the registry;
/// 4. the item must declare a target equipment slot.
///
/// If the target slot is already occupied, the previously equipped item swaps
/// back into the inventory slot.
pub fn handle_equip_item_commands(
    mut receivers: Query<
        (&mut MessageReceiver<EquipItemCommand>, &RemoteId),
        (With<Connected>, With<Joined>),
    >,
    mut players: Query<
        (
            Entity,
            &PlayerId,
            &DbPlayerId,
            &mut Inventory,
            &mut Equipment,
        ),
        With<Player>,
    >,
    registry: Res<ItemRegistry>,
    store: Res<PlayerStore>,
    runtime: Res<PersistenceRuntime>,
) {
    for (mut receiver, remote_id) in receivers.iter_mut() {
        let Some((player_entity, _, database_id, mut inventory, mut equipment)) = players
            .iter_mut()
            .find(|(_, player_id, _, _, _)| player_id.0 == remote_id.0)
        else {
            continue;
        };

        for request in receiver.receive() {
            if let Err(reason) = equip_item(
                &mut inventory,
                &mut equipment,
                &registry,
                request.slot_index,
            ) {
                bevy::log::warn!("Player {player_entity:?} equip rejected: {reason}");
                continue;
            }

            persist_inventory_and_equipment(
                &store,
                &runtime,
                database_id.0,
                &inventory,
                &equipment,
            );
        }
    }
}

/// Unequips the item in `slot` and returns it to the first free inventory slot.
pub fn handle_unequip_item_commands(
    mut receivers: Query<
        (&mut MessageReceiver<UnequipItemCommand>, &RemoteId),
        (With<Connected>, With<Joined>),
    >,
    mut players: Query<
        (
            Entity,
            &PlayerId,
            &DbPlayerId,
            &mut Inventory,
            &mut Equipment,
        ),
        With<Player>,
    >,
    store: Res<PlayerStore>,
    runtime: Res<PersistenceRuntime>,
) {
    for (mut receiver, remote_id) in receivers.iter_mut() {
        let Some((player_entity, _, database_id, mut inventory, mut equipment)) = players
            .iter_mut()
            .find(|(_, player_id, _, _, _)| player_id.0 == remote_id.0)
        else {
            continue;
        };

        for request in receiver.receive() {
            if let Err(reason) = unequip_item(&mut inventory, &mut equipment, request.slot) {
                bevy::log::warn!("Player {player_entity:?} unequip rejected: {reason}");
                continue;
            }

            persist_inventory_and_equipment(
                &store,
                &runtime,
                database_id.0,
                &inventory,
                &equipment,
            );
        }
    }
}

/// Moves (swaps) the items at `from` and `to` inventory slots.
pub fn handle_move_item_commands(
    mut receivers: Query<
        (&mut MessageReceiver<MoveItemCommand>, &RemoteId),
        (With<Connected>, With<Joined>),
    >,
    mut players: Query<(Entity, &PlayerId, &DbPlayerId, &mut Inventory), With<Player>>,
    store: Res<PlayerStore>,
    runtime: Res<PersistenceRuntime>,
) {
    for (mut receiver, remote_id) in receivers.iter_mut() {
        let Some((player_entity, _, database_id, mut inventory)) = players
            .iter_mut()
            .find(|(_, player_id, _, _)| player_id.0 == remote_id.0)
        else {
            continue;
        };

        for request in receiver.receive() {
            if let Err(reason) = move_item(&mut inventory, request.from, request.to) {
                bevy::log::warn!("Player {player_entity:?} move rejected: {reason}");
                continue;
            }

            let db_id = database_id.0;
            let current_inventory = inventory.clone();
            let repository = store.0.clone();
            runtime.0.spawn(async move {
                if let Err(e) = repository.save_inventory(db_id, &current_inventory).await {
                    bevy::log::error!("Failed to save inventory for {db_id}: {e}");
                }
            });
        }
    }
}

/// Pure mutation: equip the item at `slot_index` into its declared slot.
///
/// Returns an error reason string for logging when the command is invalid.
fn equip_item(
    inventory: &mut Inventory,
    equipment: &mut Equipment,
    registry: &ItemRegistry,
    slot_index: u8,
) -> Result<(), &'static str> {
    let slot_index = slot_index as usize;
    if slot_index >= INVENTORY_CAPACITY {
        return Err("invalid inventory slot index");
    }

    let Some(item_id) = inventory.slots[slot_index].clone() else {
        return Err("slot is empty");
    };

    let Some(item) = registry.get(&item_id) else {
        return Err("item not in registry");
    };

    let Some(target_slot) = item.config().equippable_into else {
        return Err("item is not equippable");
    };

    // The item previously occupying the target slot (if any) swaps back into
    // the inventory slot we just emptied.
    let previous = equipment.get_mut(target_slot).take();
    *equipment.get_mut(target_slot) = Some(item_id);
    inventory.slots[slot_index] = previous;

    Ok(())
}

/// Pure mutation: unequip the item in `slot` into the first free inventory
/// slot.
///
/// Returns an error reason string for logging when the command is invalid.
fn unequip_item(
    inventory: &mut Inventory,
    equipment: &mut Equipment,
    slot: EquipSlot,
) -> Result<(), &'static str> {
    let Some(item_id) = equipment.get_mut(slot).take() else {
        return Err("slot is empty");
    };

    let Some(free_slot) = inventory.slots.iter().position(|slot| slot.is_none()) else {
        // Restore the item; nothing was mutated.
        *equipment.get_mut(slot) = Some(item_id);
        return Err("inventory is full");
    };

    inventory.slots[free_slot] = Some(item_id);
    Ok(())
}

/// Pure mutation: swap the items at `from` and `to` inventory slots.
///
/// Returns an error reason string for logging when the command is invalid.
fn move_item(inventory: &mut Inventory, from: u8, to: u8) -> Result<(), &'static str> {
    let from = from as usize;
    let to = to as usize;
    if from >= INVENTORY_CAPACITY || to >= INVENTORY_CAPACITY {
        return Err("invalid inventory slot index");
    }
    inventory.slots.swap(from, to);
    Ok(())
}

/// Persists inventory and equipment together after a mutating command.
///
/// Runs on the persistence Tokio runtime, never blocking the Bevy schedule.
fn persist_inventory_and_equipment(
    store: &PlayerStore,
    runtime: &PersistenceRuntime,
    db_id: uuid::Uuid,
    inventory: &Inventory,
    equipment: &Equipment,
) {
    let repository = store.0.clone();
    let current_inventory = inventory.clone();
    let current_equipment = equipment.clone();
    runtime.0.spawn(async move {
        if let Err(e) = repository.save_inventory(db_id, &current_inventory).await {
            bevy::log::error!("Failed to save inventory for {db_id}: {e}");
        }
        if let Err(e) = repository.save_equipment(db_id, &current_equipment).await {
            bevy::log::error!("Failed to save equipment for {db_id}: {e}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevymmo_shared::items::registry::ItemId;
    use bevymmo_shared::items_impl::iron_sword::IronSword;
    use std::sync::Arc;

    fn registry_with_iron_sword() -> ItemRegistry {
        let mut registry = ItemRegistry::default();
        registry.register(Arc::new(IronSword::new()));
        registry
    }

    fn inventory_with_sword_in_slot(slot: usize) -> Inventory {
        let mut inventory = Inventory::default();
        inventory.slots[slot] = Some(ItemId::new(IronSword::ID));
        inventory
    }

    #[test]
    fn equip_moves_item_to_weapon_and_empties_slot() {
        let mut inventory = inventory_with_sword_in_slot(2);
        let mut equipment = Equipment::default();
        let registry = registry_with_iron_sword();

        assert!(equip_item(&mut inventory, &mut equipment, &registry, 2).is_ok());

        assert_eq!(equipment.weapon, Some(ItemId::new(IronSword::ID)));
        assert!(inventory.slots[2].is_none());
    }

    #[test]
    fn equip_swaps_previously_equipped_weapon_back_into_slot() {
        let mut inventory = inventory_with_sword_in_slot(2);
        let mut equipment = Equipment {
            weapon: Some(ItemId::new("old_sword")),
            ..Default::default()
        };
        let registry = registry_with_iron_sword();

        assert!(equip_item(&mut inventory, &mut equipment, &registry, 2).is_ok());

        assert_eq!(equipment.weapon, Some(ItemId::new(IronSword::ID)));
        assert_eq!(inventory.slots[2], Some(ItemId::new("old_sword")));
    }

    #[test]
    fn equip_rejects_empty_slot() {
        let mut inventory = Inventory::default();
        let mut equipment = Equipment::default();
        let registry = registry_with_iron_sword();

        assert!(equip_item(&mut inventory, &mut equipment, &registry, 0).is_err());
        assert!(equipment.weapon.is_none());
    }

    #[test]
    fn equip_rejects_out_of_bounds_slot() {
        let mut inventory = inventory_with_sword_in_slot(0);
        let mut equipment = Equipment::default();
        let registry = registry_with_iron_sword();

        assert!(equip_item(&mut inventory, &mut equipment, &registry, 99).is_err());
        assert!(equipment.weapon.is_none());
    }

    #[test]
    fn equip_rejects_unknown_item() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemId::new("not_registered"));
        let mut equipment = Equipment::default();
        let registry = registry_with_iron_sword();

        assert!(equip_item(&mut inventory, &mut equipment, &registry, 0).is_err());
        assert!(equipment.weapon.is_none());
    }

    #[test]
    fn unequip_returns_weapon_to_first_free_slot() {
        let mut inventory = Inventory::default();
        inventory.slots[3] = Some(ItemId::new("blocker"));
        let mut equipment = Equipment {
            weapon: Some(ItemId::new(IronSword::ID)),
            ..Default::default()
        };

        assert!(unequip_item(&mut inventory, &mut equipment, EquipSlot::Weapon).is_ok());

        assert!(equipment.weapon.is_none());
        // First free slot is 0: blocker stays at 3, sword lands at 0.
        assert_eq!(inventory.slots[0], Some(ItemId::new(IronSword::ID)));
        assert_eq!(inventory.slots[3], Some(ItemId::new("blocker")));
    }

    #[test]
    fn unequip_rejects_empty_weapon_slot() {
        let mut inventory = Inventory::default();
        let mut equipment = Equipment::default();

        assert!(unequip_item(&mut inventory, &mut equipment, EquipSlot::Weapon).is_err());
    }

    #[test]
    fn unequip_rejects_full_inventory_and_keeps_weapon() {
        let mut inventory = Inventory {
            slots: std::array::from_fn(|_| Some(ItemId::new("filler"))),
        };
        let sword = ItemId::new(IronSword::ID);
        let mut equipment = Equipment {
            weapon: Some(sword.clone()),
            ..Default::default()
        };

        assert!(unequip_item(&mut inventory, &mut equipment, EquipSlot::Weapon).is_err());
        assert_eq!(equipment.weapon, Some(sword));
    }

    #[test]
    fn equip_and_unequip_work_for_a_non_weapon_slot() {
        use bevymmo_shared::items_impl::leather_helmet::LeatherHelmet;

        let mut registry = registry_with_iron_sword();
        registry.register(std::sync::Arc::new(LeatherHelmet::new()));

        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemId::new(LeatherHelmet::ID));
        let mut equipment = Equipment::default();

        assert!(equip_item(&mut inventory, &mut equipment, &registry, 0).is_ok());
        assert_eq!(equipment.helmet, Some(ItemId::new(LeatherHelmet::ID)));
        assert!(inventory.slots[0].is_none());

        assert!(unequip_item(&mut inventory, &mut equipment, EquipSlot::Helmet).is_ok());
        assert!(equipment.helmet.is_none());
        assert_eq!(inventory.slots[0], Some(ItemId::new(LeatherHelmet::ID)));
    }

    #[test]
    fn move_swaps_slots() {
        let mut inventory = Inventory::default();
        inventory.slots[1] = Some(ItemId::new("a"));
        inventory.slots[5] = Some(ItemId::new("b"));

        assert!(move_item(&mut inventory, 1, 5).is_ok());

        assert_eq!(inventory.slots[1], Some(ItemId::new("b")));
        assert_eq!(inventory.slots[5], Some(ItemId::new("a")));
    }

    #[test]
    fn move_rejects_out_of_bounds() {
        let mut inventory = Inventory::default();
        assert!(move_item(&mut inventory, 0, 99).is_err());
    }
}
