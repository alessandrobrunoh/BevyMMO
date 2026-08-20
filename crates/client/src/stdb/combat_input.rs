//! Bevy input for authoritative armor casts.
//!
//! Weapon Q/W/E and 1/2/3 go through `cast_abilities_on_key` so Charge
//! abilities get a press (`eidolon_cast`) and a release (`release_cast`).
//! This system only sends armor slot/source plus the selected target.
//! Ability resolution, inscriptions, cast timing and cooldowns remain
//! server-authoritative in SpacetimeDB.

use bevy::prelude::*;
use bevymmo_gameplay::abilities::{AbilityGeometry, AbilitySlot, BaseAbilityRegistry};
use bevymmo_gameplay::items::components::Equipment;
use bevymmo_gameplay::items::registry::ItemRegistry;
use bevymmo_gameplay::items::EquipSlot;
use bevymmo_gameplay::stats::components::VitalStats;
use bevymmo_gameplay::stats::formulas::can_afford_mana;
use bevymmo_network::world_components::{NetworkEntityId, Position};

use crate::local_player::LocalPlayer;
use crate::targeting::CurrentTarget;
use crate::user_settings::{GameSettingsResource, KeyAction};

use super::commands;
use super::plugin::StdbConnection;

/// Sends one cast request for each combat action pressed during this frame.
///
/// A target is optional: the server resolves range and targeting from the
/// ability blueprint, while the selected entity/position only supplies the
/// player's intent.
pub fn send_combat_inputs(
    keyboard: Res<ButtonInput<KeyCode>>,
    settings: Res<GameSettingsResource>,
    connection: Res<StdbConnection>,
    current_target: Res<CurrentTarget>,
    target_entities: Query<(&NetworkEntityId, &Position)>,
    player: Query<(&Equipment, &VitalStats), With<LocalPlayer>>,
    item_registry: Option<Res<ItemRegistry>>,
    ability_registry: Option<Res<BaseAbilityRegistry>>,
) {
    let selected = current_target
        .entity
        .and_then(|entity| target_entities.get(entity).ok())
        .map(|(network_id, position)| (network_id.0, position.0));
    let local = player.single().ok();

    for (action, slot) in [
        (KeyAction::CastHelmet, EquipSlot::Helmet),
        (KeyAction::CastChestplate, EquipSlot::Armor),
        (KeyAction::CastBoots, EquipSlot::Shoes),
    ] {
        if !settings.just_pressed(action, &keyboard) {
            continue;
        }
        let mut geometry = None;
        if let Some((equipment, vitals)) = local {
            if let (Some(items), Some(abilities)) =
                (item_registry.as_deref(), ability_registry.as_deref())
            {
                if let Some((cost, resolved)) = armor_cast_info(equipment, slot, items, abilities) {
                    if !can_afford_mana(vitals.current_mana, cost) {
                        continue;
                    }
                    geometry = Some(resolved);
                }
            }
        }
        let selected_id = selected.map(|(id, _)| id);
        let target_entity = geometry
            .map(|geometry| geometry.selected_entity_payload(selected_id))
            .unwrap_or(selected_id);
        let target_position = selected.map(|(_, position)| position);
        let _ = commands::armor_cast(
            &connection,
            slot,
            AbilitySlot::Primary,
            target_entity,
            target_position,
        );
    }
}

fn armor_cast_info(
    equipment: &Equipment,
    slot: EquipSlot,
    items: &ItemRegistry,
    abilities: &BaseAbilityRegistry,
) -> Option<(f32, AbilityGeometry)> {
    let instance = match slot {
        EquipSlot::Helmet => equipment.helmet.as_ref(),
        EquipSlot::Armor => equipment.armor.as_ref(),
        EquipSlot::Shoes => equipment.shoes.as_ref(),
        _ => None,
    }?;
    let item = items.get(&instance.item_id)?;
    let loadout = item.ability_loadout()?;
    let ability_id =
        bevymmo_gameplay::abilities::resolve_armor_ability(loadout, &instance.ability_selection)?;
    let ability = abilities.get(ability_id)?;
    Some((ability.base_params().energy_cost, ability.geometry()))
}
