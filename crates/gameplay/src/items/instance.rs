//! `ItemInstance` — one physical item copy with its Root Word inscription state.
//!
//! `instance_id` follows the item through inventory and equipment. Weapon and
//! armor inscriptions are separate because they have different slot policies.

use serde::{Deserialize, Serialize};

use crate::abilities::{
    inscription::{ArmorInscription, WeaponInscription},
    AbilitySelection,
};

use super::registry::ItemId;

/// Identifies one physical copy of an item.
///
/// Was a `Uuid` minted client- or server-side at random. It is a database id
/// now, for two reasons: `Uuid::new_v4` needs `getrandom`, which has no backend
/// inside the SpacetimeDB WASM sandbox; and an id the client picks is not
/// authoritative. The value comes from an `#[auto_inc]` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemInstanceId(pub u64);

/// The id of an instance that has not been stored yet.
///
/// Zero is what SpacetimeDB's `#[auto_inc]` reads as "assign me one", so an
/// unsaved instance carries it until the insert comes back with the real value.
pub const UNASSIGNED_INSTANCE_ID: ItemInstanceId = ItemInstanceId(0);

impl ItemInstanceId {
    /// An id for an instance that has not been persisted yet.
    ///
    /// Replaces `new_random`. Two freshly minted instances now compare *equal*
    /// until they are stored — callers that need to tell them apart must do so
    /// by slot, or store them first.
    pub const fn unassigned() -> Self {
        UNASSIGNED_INSTANCE_ID
    }

    /// Whether this instance has been given a real id by the database.
    pub const fn is_assigned(self) -> bool {
        self.0 != 0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemInstance {
    pub instance_id: ItemInstanceId,
    pub item_id: ItemId,
    /// Which of `Item::ability_loadout()`'s Primary/Secondary options is
    /// active on THIS esemplare — `Default` (nothing picked yet) resolves to
    /// the first offered option via `abilities::resolve_active_ability`.
    #[serde(default)]
    pub ability_selection: AbilitySelection,
    /// RootWord-based weapon inscription. `None` means uninscribed.
    #[serde(default)]
    pub root_inscription: Option<WeaponInscription>,
    /// Independent inscription for armor items. Kept separate during the
    /// additive migration so armor never has to pretend to be a weapon.
    #[serde(default)]
    pub armor_inscription: Option<ArmorInscription>,
}

impl ItemInstance {
    /// Crea un nuovo esemplare senza incisione né selezione (stato
    /// "vergine"), con un `instance_id` fresco. Usato ovunque un item venga
    /// minted da zero (loot, starter kit, crafting).
    pub fn new(item_id: ItemId) -> Self {
        Self {
            instance_id: ItemInstanceId::unassigned(),
            item_id,
            ability_selection: AbilitySelection::default(),
            root_inscription: None,
            armor_inscription: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshly_minted_instances_are_unassigned_until_stored() {
        // This used to assert that two fresh instances differ, back when `new`
        // minted a random UUID. Ids are now issued by the database, so the
        // guarantee moved: a fresh instance has *no* id, and anything that
        // needs to tell two copies apart must store them first.
        let a = ItemInstance::new(ItemId::new("conduit_staff_t4"));
        let b = ItemInstance::new(ItemId::new("conduit_staff_t4"));
        assert_eq!(a.item_id, b.item_id);
        assert!(!a.instance_id.is_assigned());
        assert!(!b.instance_id.is_assigned());
    }

    #[test]
    fn an_id_from_the_database_reads_as_assigned() {
        assert!(ItemInstanceId(7).is_assigned());
        assert!(!ItemInstanceId::unassigned().is_assigned());
    }
}
