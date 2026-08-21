//! Shared crafting formulas. One implementation, used by the module tick and the UI.

use crate::items::components::Inventory;
use crate::items::instance::{ItemInstance, ItemInstanceId};
use crate::items::recipe::CraftRecipe;
use crate::items::registry::ItemId;

/// Why a craft was refused. The bag is left unchanged on preview failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CraftError {
    QuantityZero,
    MissingIngredient {
        item_id: ItemId,
        need: u32,
        have: u32,
    },
    InventoryFull,
}

impl CraftError {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::QuantityZero => "quantity must be at least 1",
            Self::MissingIngredient { .. } => "missing crafting materials",
            Self::InventoryFull => "inventory is full",
        }
    }
}

impl std::fmt::Display for CraftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingIngredient {
                item_id,
                need,
                have,
            } => write!(f, "missing {}: need {need} have {have}", item_id.as_str()),
            other => f.write_str(other.as_str()),
        }
    }
}

impl std::error::Error for CraftError {}

/// Ingredients required for `quantity` outputs.
pub fn scaled_cost(recipe: &CraftRecipe, quantity: u32) -> Vec<(ItemId, u32)> {
    recipe
        .ingredients
        .iter()
        .map(|ingredient| {
            (
                ingredient.item_id.clone(),
                ingredient.amount.saturating_mul(quantity),
            )
        })
        .collect()
}

/// Channel duration for a whole batch: authored seconds times quantity.
pub fn channel_duration(recipe: &CraftRecipe, quantity: u32) -> f32 {
    recipe.channel_seconds.max(0.0) * quantity as f32
}

/// A craft that has been checked against a bag and is ready to apply.
#[derive(Debug, Clone, PartialEq)]
pub struct CraftPlan {
    pub output_id: ItemId,
    pub quantity: u32,
    pub costs: Vec<(ItemId, u32)>,
    pub channel_seconds: f32,
}

/// Largest quantity the bag can afford and still hold, or 0.
pub fn max_craftable(
    inventory: &Inventory,
    recipe: &CraftRecipe,
    output_id: &ItemId,
    output_stacks: bool,
) -> u32 {
    let mut max_from_mats = u32::MAX;
    for ingredient in &recipe.ingredients {
        if ingredient.amount == 0 {
            continue;
        }
        let have = inventory.count_item(&ingredient.item_id);
        max_from_mats = max_from_mats.min(have / ingredient.amount);
    }
    if max_from_mats == 0 || max_from_mats == u32::MAX {
        return 0;
    }
    let mut hi = max_from_mats;
    let mut lo = 0u32;
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if preview_craft(inventory, recipe, output_id, output_stacks, mid).is_ok() {
            lo = mid;
        } else {
            hi = mid.saturating_sub(1);
        }
    }
    lo
}

/// Checks materials and bag space without mutating `inventory`.
pub fn preview_craft(
    inventory: &Inventory,
    recipe: &CraftRecipe,
    output_id: &ItemId,
    output_stacks: bool,
    quantity: u32,
) -> Result<CraftPlan, CraftError> {
    if quantity == 0 {
        return Err(CraftError::QuantityZero);
    }
    let costs = scaled_cost(recipe, quantity);
    for (item_id, need) in &costs {
        let have = inventory.count_item(item_id);
        if have < *need {
            return Err(CraftError::MissingIngredient {
                item_id: item_id.clone(),
                need: *need,
                have,
            });
        }
    }

    let mut trial = inventory.clone();
    for (item_id, need) in &costs {
        trial
            .remove_item_amount(item_id, *need)
            .map_err(|_| CraftError::MissingIngredient {
                item_id: item_id.clone(),
                need: *need,
                have: inventory.count_item(item_id),
            })?;
    }
    if trial.space_for(output_id, output_stacks) < quantity {
        return Err(CraftError::InventoryFull);
    }

    Ok(CraftPlan {
        output_id: output_id.clone(),
        quantity,
        costs,
        channel_seconds: channel_duration(recipe, quantity),
    })
}

/// Consumes ingredients and grants `plan.quantity` outputs. Callers must
/// [`preview_craft`] first; this still re-checks so a stale plan cannot
/// overdraft the bag.
pub fn apply_craft(
    inventory: &mut Inventory,
    plan: &CraftPlan,
    output_stacks: bool,
    mut mint: impl FnMut() -> ItemInstanceId,
) -> Result<(), CraftError> {
    for (item_id, need) in &plan.costs {
        let have = inventory.count_item(item_id);
        if have < *need {
            return Err(CraftError::MissingIngredient {
                item_id: item_id.clone(),
                need: *need,
                have,
            });
        }
        inventory.remove_item_amount(item_id, *need).map_err(|_| {
            CraftError::MissingIngredient {
                item_id: item_id.clone(),
                need: *need,
                have,
            }
        })?;
    }

    if output_stacks {
        let added = inventory.add_stackable(plan.output_id.clone(), plan.quantity, &mut mint);
        if added < plan.quantity {
            return Err(CraftError::InventoryFull);
        }
        return Ok(());
    }

    for _ in 0..plan.quantity {
        let Some(free) = inventory.slots.iter().position(Option::is_none) else {
            return Err(CraftError::InventoryFull);
        };
        let mut instance = ItemInstance::new(plan.output_id.clone());
        instance.instance_id = mint();
        inventory.slots[free] = Some(instance);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::instance::ItemInstance;
    use crate::items::recipe::CraftIngredient;

    fn wood() -> ItemId {
        ItemId::new("wood")
    }

    fn copper() -> ItemId {
        ItemId::new("copper")
    }

    fn sword() -> ItemId {
        ItemId::new("sword")
    }

    fn recipe() -> CraftRecipe {
        CraftRecipe {
            ingredients: vec![
                CraftIngredient {
                    item_id: wood(),
                    amount: 2,
                },
                CraftIngredient {
                    item_id: copper(),
                    amount: 4,
                },
            ],
            channel_seconds: 3.0,
        }
    }

    fn pile(id: ItemId, instance: u64, quantity: u32) -> ItemInstance {
        let mut item = ItemInstance::new(id);
        item.instance_id = ItemInstanceId(instance);
        item.quantity = quantity;
        item
    }

    fn bag_with(wood_qty: u32, copper_qty: u32) -> Inventory {
        let mut inv = Inventory::default();
        if wood_qty > 0 {
            inv.slots[0] = Some(pile(wood(), 1, wood_qty));
        }
        if copper_qty > 0 {
            inv.slots[1] = Some(pile(copper(), 2, copper_qty));
        }
        inv
    }

    #[test]
    fn quantity_two_scales_cost_and_time() {
        let costs = scaled_cost(&recipe(), 2);
        assert_eq!(costs, vec![(wood(), 4), (copper(), 8)]);
        assert_eq!(channel_duration(&recipe(), 2), 6.0);
    }

    #[test]
    fn preview_refuses_quantity_zero() {
        let inv = bag_with(10, 10);
        assert_eq!(
            preview_craft(&inv, &recipe(), &sword(), false, 0).unwrap_err(),
            CraftError::QuantityZero
        );
    }

    #[test]
    fn preview_reports_short_materials() {
        let inv = bag_with(10, 1);
        match preview_craft(&inv, &recipe(), &sword(), false, 1).unwrap_err() {
            CraftError::MissingIngredient {
                item_id,
                need,
                have,
            } => {
                assert_eq!(item_id.as_str(), "copper");
                assert_eq!(need, 4);
                assert_eq!(have, 1);
            }
            other => panic!("expected missing copper, got {other:?}"),
        }
    }

    #[test]
    fn preview_refuses_when_unique_output_has_no_free_slot() {
        let mut inv = bag_with(10, 10);
        for slot in inv.slots.iter_mut().skip(2) {
            *slot = Some(pile(ItemId::new("rock"), 9, 1));
        }
        assert_eq!(
            preview_craft(&inv, &recipe(), &sword(), false, 1).unwrap_err(),
            CraftError::InventoryFull
        );
    }

    #[test]
    fn preview_succeeds_after_consuming_a_full_stack_frees_a_slot() {
        let mut inv = Inventory::default();
        inv.slots[0] = Some(pile(wood(), 1, 2));
        inv.slots[1] = Some(pile(copper(), 2, 4));
        for slot in inv.slots.iter_mut().skip(2) {
            *slot = Some(pile(ItemId::new("rock"), 9, 1));
        }
        preview_craft(&inv, &recipe(), &sword(), false, 1).expect("emptying copper frees a slot");
    }

    #[test]
    fn apply_deducts_materials_and_inserts_uninscribed_swords() {
        let mut inv = bag_with(10, 10);
        let plan = preview_craft(&inv, &recipe(), &sword(), false, 2).expect("affordable");
        assert_eq!(plan.channel_seconds, 6.0);
        let mut next = 10u64;
        apply_craft(&mut inv, &plan, false, || {
            let id = ItemInstanceId(next);
            next += 1;
            id
        })
        .expect("apply");
        assert_eq!(inv.count_item(&wood()), 6);
        assert_eq!(inv.count_item(&copper()), 2);
        assert_eq!(inv.count_item(&sword()), 2);
        let swords: Vec<_> = inv
            .slots
            .iter()
            .flatten()
            .filter(|item| item.item_id == sword())
            .collect();
        assert_eq!(swords.len(), 2);
        assert!(swords.iter().all(|item| item.root_inscription.is_none()));
        assert!(swords.iter().all(|item| item.quantity == 1));
    }

    #[test]
    fn max_craftable_is_limited_by_the_shortest_ingredient() {
        let inv = bag_with(10, 9);
        // 10/2 = 5 wood, 9/4 = 2 copper
        assert_eq!(max_craftable(&inv, &recipe(), &sword(), false), 2);
    }
}
