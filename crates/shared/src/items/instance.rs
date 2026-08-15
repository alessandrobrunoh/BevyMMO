//! `ItemInstance` — un esemplare fisico di un item, distinto dal semplice
//! riferimento di catalogo `ItemId`.
//!
//! Fino a questo punto un `ItemId` bastava: gli item non avevano stato
//! proprio (decisione esplicita "1 item = 1 slot", nessuna istanza). Da
//! quando un'arma può portare una propria [`crate::abilities::WeaponInscriptions`]
//! incisa dal giocatore, due copie dello stesso tipo di arma (due Flame
//! Staff) devono poter essere diverse fra loro — serve un identificatore
//! stabile per esemplare, non solo per tipo.
//!
//! `instance_id` segue l'oggetto ovunque vada (inventario, equipaggiato,
//! eventualmente scambiato/droppato in futuro); `inscriptions` è `None` per
//! qualunque item che non ha `weapon_abilities()` nel proprio catalogo
//! (armor, pozioni, ...).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::abilities::{AbilitySelection, WeaponInscriptions};

use super::registry::ItemId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemInstanceId(pub Uuid);

impl ItemInstanceId {
    pub fn new_random() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemInstance {
    pub instance_id: ItemInstanceId,
    pub item_id: ItemId,
    pub inscriptions: Option<WeaponInscriptions>,
    /// Which of `Item::weapon_abilities()`'s Primary/Secondary options is
    /// active on THIS esemplare — `Default` (nothing picked yet) resolves to
    /// the first offered option via `abilities::resolve_active_ability`.
    #[serde(default)]
    pub ability_selection: AbilitySelection,
}

impl ItemInstance {
    /// Crea un nuovo esemplare senza incisione né selezione (stato
    /// "vergine"), con un `instance_id` fresco. Usato ovunque un item venga
    /// minted da zero (loot, starter kit, crafting).
    pub fn new(item_id: ItemId) -> Self {
        Self {
            instance_id: ItemInstanceId::new_random(),
            item_id,
            inscriptions: None,
            ability_selection: AbilitySelection::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_freshly_minted_instances_of_the_same_item_have_different_ids() {
        let a = ItemInstance::new(ItemId::new("magic_staff"));
        let b = ItemInstance::new(ItemId::new("magic_staff"));
        assert_eq!(a.item_id, b.item_id);
        assert_ne!(a.instance_id, b.instance_id);
    }
}
