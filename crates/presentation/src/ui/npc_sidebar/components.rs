//! Componenti per la sidebar NPC (pannello informativo su click NPC).

use bevy::prelude::*;
use bevymmo_gameplay::items::registry::ItemId;

/// Marker per la sidebar NPC attualmente aperta.
///
/// Traccia l'entity target della sidebar corrente per poterla aggiornare o
/// rimuovere quando necessario.
#[derive(Component, Debug)]
pub struct NpcSidebar {
    /// L'entity del NPC che questa sidebar sta mostrando.
    pub target: Entity,
}

/// Button for claiming one catalogue item from the NPC vendor.
#[derive(Component, Debug, Clone)]
pub struct VendorItemButton {
    pub npc: Entity,
    pub item_id: ItemId,
}
