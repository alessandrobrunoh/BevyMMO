//! Row spellings of the domain types, and the conversions to and from them.
//!
//! `bevymmo_domain` carries no SATS derives — its newtypes would panic the
//! derive, and the derive assumes it lives in a module crate (see that crate's
//! `lib.rs`). So every domain type that has to be stored gets a mirror here with
//! named fields, plus `From` in both directions.
//!
//! The mirrors are deliberately flat: ids become `String`, `Cow` disappears,
//! fixed-size arrays become `Vec`. SATS has no impl for `[T; N]`, `Cow` or
//! `HashMap`, so those would not survive the trip regardless.
//!
//! Note what is *not* here: JSON. The Postgres schema stored inventories,
//! equipment and glyphs as JSON in `TEXT` columns, which meant the database
//! could not see inside them. These are real columns, so `spacetime sql` can.

use bevymmo_domain::abilities::inscription::{Inscription, WeaponInscriptions};
use bevymmo_domain::abilities::known_glyphs::KnownGlyphs;
use bevymmo_domain::abilities::weapon_abilities::AbilitySelection;
use bevymmo_domain::abilities::{AbilityId, AncientWordId, EssenceId, ModifierId};
use bevymmo_domain::items::components::{Equipment, Inventory};
use bevymmo_domain::items::instance::{ItemInstance, ItemInstanceId};
use bevymmo_domain::items::registry::ItemId;
use bevymmo_domain::items::EquipSlot;
use bevymmo_domain::spells::components::SpellHotbar;
use bevymmo_domain::spells::registry::SpellId;
use bevymmo_domain::stats::components::{CombatStats, MovementStats, StatsBundleData, VitalStats};
use glam::Vec3;
use spacetimedb::SpacetimeType;

/// A vector as a database column.
#[derive(SpacetimeType, Clone, Copy, Debug, PartialEq, Default)]
pub struct Vec3Row {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl From<Vec3> for Vec3Row {
    fn from(v: Vec3) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}

impl From<Vec3Row> for Vec3 {
    fn from(v: Vec3Row) -> Self {
        Vec3::new(v.x, v.y, v.z)
    }
}

/// The seven numbers that make up a character's stats.
///
/// Stored as *base* values, without equipment bonuses. The Bevy server was
/// careful about this too — it persisted `base_stats_without_equipment` so that
/// re-equipping on login would not compound the bonuses — and the distinction
/// has to survive: effective stats are derived, never stored.
#[derive(SpacetimeType, Clone, Copy, Debug, PartialEq)]
pub struct StatsRow {
    pub current_health: f32,
    pub max_health: f32,
    pub max_mana: f32,
    pub mana_regeneration: f32,
    pub armor: f32,
    pub movement_speed: f32,
    pub attack_power: f32,
}

impl From<&StatsBundleData> for StatsRow {
    fn from(s: &StatsBundleData) -> Self {
        Self {
            current_health: s.vital.current_health,
            max_health: s.vital.max_health,
            max_mana: s.vital.max_mana,
            mana_regeneration: s.vital.mana_regeneration,
            armor: s.combat.armor,
            movement_speed: s.movement.speed,
            attack_power: s.combat.attack_power,
        }
    }
}

impl From<StatsRow> for StatsBundleData {
    fn from(s: StatsRow) -> Self {
        StatsBundleData {
            vital: VitalStats {
                current_health: s.current_health,
                max_health: s.max_health,
                max_mana: s.max_mana,
                mana_regeneration: s.mana_regeneration,
            },
            combat: CombatStats {
                armor: s.armor,
                attack_power: s.attack_power,
            },
            movement: MovementStats {
                speed: s.movement_speed,
            },
        }
    }
}

#[derive(SpacetimeType, Clone, Debug, PartialEq, Default)]
pub struct InscriptionRow {
    pub essence: Option<String>,
    pub modifiers: Vec<String>,
    pub ancient_word: Option<String>,
}

impl From<&Inscription> for InscriptionRow {
    fn from(i: &Inscription) -> Self {
        Self {
            essence: i.essence.as_ref().map(|e| e.as_str().to_string()),
            modifiers: i.modifiers.iter().map(|m| m.as_str().to_string()).collect(),
            ancient_word: i.ancient_word.as_ref().map(|w| w.as_str().to_string()),
        }
    }
}

impl From<&InscriptionRow> for Inscription {
    fn from(i: &InscriptionRow) -> Self {
        Inscription {
            essence: i.essence.clone().map(EssenceId::new),
            modifiers: i.modifiers.iter().cloned().map(ModifierId::new).collect(),
            ancient_word: i.ancient_word.clone().map(AncientWordId::new),
        }
    }
}

#[derive(SpacetimeType, Clone, Debug, PartialEq, Default)]
pub struct WeaponInscriptionsRow {
    pub primary: InscriptionRow,
    pub secondary: InscriptionRow,
    pub ultimate: InscriptionRow,
}

impl From<&WeaponInscriptions> for WeaponInscriptionsRow {
    fn from(w: &WeaponInscriptions) -> Self {
        Self {
            primary: (&w.primary).into(),
            secondary: (&w.secondary).into(),
            ultimate: (&w.ultimate).into(),
        }
    }
}

impl From<&WeaponInscriptionsRow> for WeaponInscriptions {
    fn from(w: &WeaponInscriptionsRow) -> Self {
        WeaponInscriptions {
            primary: (&w.primary).into(),
            secondary: (&w.secondary).into(),
            ultimate: (&w.ultimate).into(),
        }
    }
}

#[derive(SpacetimeType, Clone, Debug, PartialEq, Default)]
pub struct AbilitySelectionRow {
    pub primary: Option<String>,
    pub secondary: Option<String>,
}

impl From<&AbilitySelection> for AbilitySelectionRow {
    fn from(a: &AbilitySelection) -> Self {
        Self {
            primary: a.primary.as_ref().map(|id| id.as_str().to_string()),
            secondary: a.secondary.as_ref().map(|id| id.as_str().to_string()),
        }
    }
}

impl From<&AbilitySelectionRow> for AbilitySelection {
    fn from(a: &AbilitySelectionRow) -> Self {
        AbilitySelection {
            primary: a.primary.clone().map(AbilityId::new),
            secondary: a.secondary.clone().map(AbilityId::new),
        }
    }
}

/// One physical copy of an item.
#[derive(SpacetimeType, Clone, Debug, PartialEq)]
pub struct ItemInstanceRow {
    /// Zero means "not stored yet"; see [`ItemInstanceId`].
    pub instance_id: u64,
    pub item_id: String,
    pub inscriptions: Option<WeaponInscriptionsRow>,
    pub ability_selection: AbilitySelectionRow,
}

impl From<&ItemInstance> for ItemInstanceRow {
    fn from(i: &ItemInstance) -> Self {
        Self {
            instance_id: i.instance_id.0,
            item_id: i.item_id.as_str().to_string(),
            inscriptions: i.inscriptions.as_ref().map(Into::into),
            ability_selection: (&i.ability_selection).into(),
        }
    }
}

impl From<&ItemInstanceRow> for ItemInstance {
    fn from(i: &ItemInstanceRow) -> Self {
        ItemInstance {
            instance_id: ItemInstanceId(i.instance_id),
            item_id: ItemId::new(i.item_id.clone()),
            inscriptions: i.inscriptions.as_ref().map(Into::into),
            ability_selection: (&i.ability_selection).into(),
        }
    }
}

/// The ten equipment slots, in the order [`EquipSlot`] declares them.
pub const EQUIP_SLOTS: [EquipSlot; 10] = [
    EquipSlot::Bag,
    EquipSlot::Helmet,
    EquipSlot::Cape,
    EquipSlot::Weapon,
    EquipSlot::Armor,
    EquipSlot::Offhand,
    EquipSlot::Potion,
    EquipSlot::Shoes,
    EquipSlot::Food,
    EquipSlot::Mount,
];

/// Converts an inventory to its stored slot list.
///
/// `Inventory` is `[Option<ItemInstance>; 10]` and SATS has no impl for
/// fixed-size arrays, so the length is carried by convention. Reading back
/// tolerates a short or long list rather than panicking: a schema change that
/// alters the slot count should degrade, not crash the module.
pub fn inventory_to_rows(inventory: &Inventory) -> Vec<Option<ItemInstanceRow>> {
    inventory
        .slots
        .iter()
        .map(|slot| slot.as_ref().map(Into::into))
        .collect()
}

pub fn inventory_from_rows(rows: &[Option<ItemInstanceRow>]) -> Inventory {
    let mut inventory = Inventory::default();
    for (slot, row) in inventory.slots.iter_mut().zip(rows) {
        *slot = row.as_ref().map(Into::into);
    }
    inventory
}

pub fn equipment_to_rows(equipment: &Equipment) -> Vec<Option<ItemInstanceRow>> {
    EQUIP_SLOTS
        .iter()
        .map(|slot| equipment.get(*slot).as_ref().map(Into::into))
        .collect()
}

pub fn equipment_from_rows(rows: &[Option<ItemInstanceRow>]) -> Equipment {
    let mut equipment = Equipment::default();
    for (slot, row) in EQUIP_SLOTS.iter().zip(rows) {
        *equipment.get_mut(*slot) = row.as_ref().map(Into::into);
    }
    equipment
}

/// The three hotbar slots as stored.
#[derive(SpacetimeType, Clone, Debug, PartialEq, Default)]
pub struct HotbarRow {
    pub q: Option<String>,
    pub w: Option<String>,
    pub e: Option<String>,
}

impl From<&SpellHotbar> for HotbarRow {
    fn from(h: &SpellHotbar) -> Self {
        Self {
            q: h.q_spell.as_ref().map(|s| s.as_str().to_string()),
            w: h.w_spell.as_ref().map(|s| s.as_str().to_string()),
            e: h.e_spell.as_ref().map(|s| s.as_str().to_string()),
        }
    }
}

impl From<&HotbarRow> for SpellHotbar {
    fn from(h: &HotbarRow) -> Self {
        SpellHotbar {
            q_spell: h.q.clone().map(SpellId::new),
            w_spell: h.w.clone().map(SpellId::new),
            e_spell: h.e.clone().map(SpellId::new),
        }
    }
}

pub fn known_glyphs_from_rows(
    essences: &[String],
    modifiers: &[String],
    ancient_words: &[String],
) -> KnownGlyphs {
    KnownGlyphs {
        essences: essences.iter().cloned().map(EssenceId::new).collect(),
        modifiers: modifiers.iter().cloned().map(ModifierId::new).collect(),
        ancient_words: ancient_words
            .iter()
            .cloned()
            .map(AncientWordId::new)
            .collect(),
    }
}
