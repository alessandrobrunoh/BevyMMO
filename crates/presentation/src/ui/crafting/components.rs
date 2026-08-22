//! Markers for the crafter list and confirm cards.

use bevy::prelude::*;
use bevymmo_gameplay::items::registry::ItemId;

/// Open crafter catalogue card.
#[derive(Component, Debug)]
pub struct CraftListCard {
    pub npc: Entity,
}

/// Button that opens the confirm dialog for one recipe.
#[derive(Component, Debug, Clone)]
pub struct CraftRecipeButton {
    pub npc: Entity,
    pub item_id: ItemId,
}

/// Confirm dialog for a single recipe. Quantity is 1 in v1.
#[derive(Component, Debug)]
pub struct CraftDialogCard {
    pub npc: Entity,
    pub item_id: ItemId,
}

/// Confirm button on the dialog.
#[derive(Component, Debug, Clone)]
pub struct CraftSubmitButton {
    pub npc: Entity,
    pub item_id: ItemId,
}
