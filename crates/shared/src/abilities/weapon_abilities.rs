//! `WeaponAbilities` — i tre gesti fissi di una variante d'arma. Vive nel
//! catalogo (`Item::weapon_abilities`), quindi salva `AbilityId` (riferimenti
//! al registry) e non `Arc<dyn BaseAbility>`, esattamente come `Equipment`
//! salva `ItemId` e non `Arc<dyn Item>`.

use serde::{Deserialize, Serialize};

use super::base_ability::AbilityId;
use super::slot::AbilitySlot;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeaponAbilities {
    pub primary: AbilityId,
    pub secondary: AbilityId,
    pub ultimate: AbilityId,
}

impl WeaponAbilities {
    pub fn get(&self, slot: AbilitySlot) -> &AbilityId {
        match slot {
            AbilitySlot::Primary => &self.primary,
            AbilitySlot::Secondary => &self.secondary,
            AbilitySlot::Ultimate => &self.ultimate,
        }
    }
}
