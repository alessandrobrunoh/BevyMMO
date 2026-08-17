//! Inventory, equipment, hotbar selection and Eidolon inscriptions.
//!
//! Ported from `crates/server/src/items` (`systems.rs`, `bonuses.rs`,
//! `available_spells.rs`) and the three request handlers that lived in
//! `crates/server/src/network/server.rs`.
//!
//! # What changed against the Bevy server
//!
//! - **No anti-spoofing checks.** Every Bevy handler started by scanning the
//!   player query for the entity whose `PlayerId` matched the sending peer,
//!   precisely so a client could not name someone else's entity in the command.
//!   Here the row key *is* `ctx.sender()`, verified by SpacetimeDB, so there is
//!   nothing to spoof and nothing to check — the lookup and the authorisation
//!   are the same operation.
//! - **Rejections are returned, not logged.** The Bevy handlers `warn!`ed and
//!   dropped an invalid command because a message receiver has no reply
//!   channel. A reducer's `Err` reaches the caller, so an invalid request now
//!   tells the player *why* instead of silently doing nothing.
//! - **No explicit persistence step.** `persist_inventory_and_equipment` (which
//!   spawned Tokio tasks against Postgres and could lose the write on a crash)
//!   has no equivalent: writing the row *is* persisting it, inside the same
//!   transaction as the mutation.
//! - **Derived state is recomputed by an explicit call**, not by a
//!   `Changed<Equipment>` query. `recompute_equipment_bonuses` and
//!   `recompute_available_spells` both ran reactively; there is no change
//!   detection here, so every reducer that touches `equipment` calls
//!   [`recompute_effective_stats`] and [`prune_hotbar_to_available`] itself.
//!
//! # Base versus effective stats
//!
//! `player_stats` holds the character's stats **without** equipment bonuses —
//! the same distinction the Bevy server kept with
//! `bonuses::base_stats_without_equipment`, and for the same reason: if the
//! stored value already contained the bonus, re-applying it on the next login
//! (or the next equip) would compound it. `entity_stats` holds the derived,
//! effective value that combat and the client read. Nothing writes bonuses back
//! into `player_stats`.

use std::collections::HashSet;
use std::sync::OnceLock;

use bevymmo_domain::abilities::inscription::{
    validate_weapon_inscriptions, ArmorInscription, Inscription, SecondaryWord, SlotInscription,
    WeaponInscription,
};
use bevymmo_domain::abilities::{
    resolve_active_ability, AbilityId, AbilitySlot, AncientWordId, AncientWordRegistry,
    BaseAbilityRegistry, EssenceId,
    EssenceRegistry, ModifierId, ModifierRegistry, RootWordId, RootWordRegistry,
};
use bevymmo_domain::items::components::{EquipSlot, Equipment, Inventory, INVENTORY_CAPACITY};
use bevymmo_domain::items::definition::EquipRequirement;
use bevymmo_domain::items::instance::{ItemInstance, ItemInstanceId};
use bevymmo_domain::items::registry::{ItemId, ItemRegistry};
use bevymmo_domain::items::{compute_available_choices, AvailableSpellChoices};
use bevymmo_domain::spells::components::{HotbarSlot, SpellHotbar};
use bevymmo_domain::spells::registry::SpellId;
use spacetimedb::{reducer, Identity, ReducerContext, Table};

use crate::rows::{
    equipment_from_rows, equipment_to_rows, inventory_from_rows, inventory_to_rows,
    known_ancient_language_from_rows, known_glyphs_from_rows, HotbarRow,
};
use crate::tables::{
    equipment, hotbar, inventory, known_ancient_language, known_glyphs, player, EquipmentTable,
    Hotbar, InventoryTable,
};

// ---------------------------------------------------------------------------
// Content registries
// ---------------------------------------------------------------------------
//
// The Bevy server built these once at `Startup` and handed them around as
// `Res<...>`. A module has no startup schedule and no resources, so they are
// process-wide statics built on first use instead. They must not be rebuilt per
// call: `default_items()` allocates a dozen `Arc`s and a `HashMap`, and every
// equip would pay for it.
//
// `OnceLock` rather than a plain `static`: the registries own trait objects, so
// they cannot be `const`-constructed. It is sound here for the same reason it is
// anywhere — the contents are immutable after initialisation.

/// Every item this build ships.
fn item_registry() -> &'static ItemRegistry {
    static REGISTRY: OnceLock<ItemRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::content::items::default_items)
}

/// The Eidolon gestures (`BaseAbility`) items can offer.
fn ability_registry() -> &'static BaseAbilityRegistry {
    static REGISTRY: OnceLock<BaseAbilityRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::content::abilities::default_base_abilities)
}

fn essence_registry() -> &'static EssenceRegistry {
    static REGISTRY: OnceLock<EssenceRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::content::essences::default_essences)
}

fn modifier_registry() -> &'static ModifierRegistry {
    static REGISTRY: OnceLock<ModifierRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::content::modifiers::default_modifiers)
}

fn ancient_word_registry() -> &'static AncientWordRegistry {
    static REGISTRY: OnceLock<AncientWordRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::content::ancient_words::default_ancient_words)
}

fn root_word_registry() -> &'static RootWordRegistry {
    static REGISTRY: OnceLock<RootWordRegistry> = OnceLock::new();
    REGISTRY.get_or_init(bevymmo_domain::content::root_words::default_root_words)
}

// ---------------------------------------------------------------------------
// Reducers
// ---------------------------------------------------------------------------

/// Equips the item held in inventory slot `slot_index` into the slot its
/// catalogue entry declares.
///
/// Validation order, unchanged from `items::systems::equip_item`: the index must
/// be in range, the slot must hold something, the item must exist in the
/// registry, and it must declare an `equippable_into`. What the Bevy version did
/// *not* do is check `Item::equip_requirements` — the hook existed and was never
/// read — so that check is added here (see [`check_equip_requirements`]).
///
/// If the target equipment slot is already occupied, the item that was there
/// swaps back into the inventory slot this one just left; the inventory can
/// therefore never overflow, which is why equipping has no "inventory full"
/// failure the way unequipping does.
#[reducer]
pub fn equip_item(ctx: &ReducerContext, slot_index: u8) -> Result<(), String> {
    // The caller *is* the character. No `PlayerId`-to-entity scan, and no
    // "is this really your entity" check: see the module docs.
    let identity = ctx.sender();
    let mut inventory = load_inventory(ctx, identity)?;
    let mut equipment = load_equipment(ctx, identity)?;

    let index = usize::from(slot_index);
    if index >= INVENTORY_CAPACITY {
        return Err(format!(
            "inventory slot {slot_index} out of range (0..{INVENTORY_CAPACITY})"
        ));
    }

    let Some(mut instance) = inventory.slots[index].clone() else {
        return Err(format!("inventory slot {slot_index} is empty"));
    };

    let item = item_registry()
        .get(&instance.item_id)
        .ok_or_else(|| format!("unknown item {:?}", instance.item_id.as_str()))?;

    let target = item
        .config()
        .equippable_into
        .ok_or_else(|| format!("{:?} is not equippable", instance.item_id.as_str()))?;

    check_equip_requirements(item.equip_requirements())?;

    // An esemplare that has never been stored carries id 0. Give it one now:
    // from here on it can hold an Incisione, and an inscription that cannot be
    // told apart from another copy's is worse than useless.
    if !instance.instance_id.is_assigned() {
        instance.instance_id = ItemInstanceId(next_instance_id(ctx));
    }

    let previous = equipment.get_mut(target).take();
    *equipment.get_mut(target) = Some(instance);
    inventory.slots[index] = previous;

    store_inventory(ctx, identity, &inventory);
    store_equipment(ctx, identity, &equipment);

    // Equipment changed, so both things derived from it are now stale.
    recompute_effective_stats(ctx, identity)?;
    prune_hotbar_to_available(ctx, identity, &equipment);
    Ok(())
}

/// Unequips `slot` and returns the item to the first free inventory slot.
///
/// `slot` is the equipment slot's name, case-insensitive (`"weapon"`,
/// `"helmet"`, ... — see [`parse_equip_slot`]). A string rather than an enum
/// because the reducer signature is the client-facing API and a name is
/// readable in `spacetime call` and in logs; the parse rejects anything else.
///
/// Fails, and changes nothing, when the inventory is full — same as the Bevy
/// version, which restored the item before returning the error.
#[reducer]
pub fn unequip_item(ctx: &ReducerContext, slot: String) -> Result<(), String> {
    let identity = ctx.sender();
    let target = parse_equip_slot(&slot)?;
    let mut inventory = load_inventory(ctx, identity)?;
    let mut equipment = load_equipment(ctx, identity)?;

    let Some(instance) = equipment.get_mut(target).take() else {
        return Err(format!("equipment slot {slot:?} is empty"));
    };

    let Some(free) = inventory.slots.iter().position(Option::is_none) else {
        // Nothing has been written yet, so "restoring" is only a matter of not
        // storing the local copy — but put it back anyway so the local state
        // stays truthful if this function ever grows a later step.
        *equipment.get_mut(target) = Some(instance);
        return Err("inventory is full".to_string());
    };

    inventory.slots[free] = Some(instance);

    store_inventory(ctx, identity, &inventory);
    store_equipment(ctx, identity, &equipment);

    recompute_effective_stats(ctx, identity)?;
    prune_hotbar_to_available(ctx, identity, &equipment);
    Ok(())
}

/// Swaps the contents of two inventory slots.
///
/// Purely positional: nothing derived depends on *where* in the inventory an
/// item sits, so unlike equip/unequip this touches no stats and no hotbar.
#[reducer]
pub fn move_item(ctx: &ReducerContext, from: u8, to: u8) -> Result<(), String> {
    let identity = ctx.sender();
    let (from_index, to_index) = (usize::from(from), usize::from(to));
    if from_index >= INVENTORY_CAPACITY || to_index >= INVENTORY_CAPACITY {
        return Err(format!(
            "inventory slots {from}/{to} out of range (0..{INVENTORY_CAPACITY})"
        ));
    }

    let mut inventory = load_inventory(ctx, identity)?;
    inventory.slots.swap(from_index, to_index);
    store_inventory(ctx, identity, &inventory);
    Ok(())
}

/// Binds a spell to a hotbar key, or clears the key when `spell_id` is `None`.
///
/// `slot` is `"q"`, `"w"` or `"e"`, case-insensitive.
///
/// # Why not `set_hotbar_slot`
///
/// `reducers::spells` still carries a `set_hotbar_slot` stub, and two reducers
/// cannot share a name — the generated registration symbols collide at link
/// time. This one is named for what it does instead. **When that stub goes
/// away, the right move is to rename this reducer to `set_hotbar_slot`**, or to
/// have the stub delegate to [`assign_hotbar_spell`], which is public for
/// exactly that reason.
#[reducer]
pub fn set_hotbar_spell(
    ctx: &ReducerContext,
    slot: String,
    spell_id: Option<String>,
) -> Result<(), String> {
    assign_hotbar_spell(ctx, ctx.sender(), &slot, spell_id)
}

/// The body of [`set_hotbar_spell`], callable from another reducer.
///
/// The legal picks are **not** "any registered spell": they are whatever the
/// currently equipped items offer for that key, unioned from every equipped
/// item's `SpellKit` by
/// [`compute_available_choices`](bevymmo_domain::items::compute_available_choices).
/// That is the same rule `handle_update_hotbar_slot_requests` enforced against
/// the reactively-maintained `AvailableSpellChoices` component; here the value
/// is computed on the spot, because there is no component to keep in sync and
/// the computation is a walk over ten slots.
pub fn assign_hotbar_spell(
    ctx: &ReducerContext,
    identity: Identity,
    slot: &str,
    spell_id: Option<String>,
) -> Result<(), String> {
    let key = parse_hotbar_slot(slot)?;
    let equipment = load_equipment(ctx, identity)?;

    let row = ctx
        .db
        .hotbar()
        .identity()
        .find(&identity)
        .ok_or_else(|| "no character for this identity; call `join` first".to_string())?;
    let mut bar = SpellHotbar::from(&row.slots);

    let spell = spell_id.map(SpellId::new);
    if let Some(id) = &spell {
        let choices = available_choices(&equipment);
        if !choices.contains(key, id) {
            return Err(format!(
                "{:?} is not offered on {slot:?} by your equipped items",
                id.as_str()
            ));
        }
    }

    bar.assign(key, spell);
    ctx.db.hotbar().identity().update(Hotbar {
        identity,
        slots: HotbarRow::from(&bar),
    });
    Ok(())
}

/// Inscribes one slot (`"primary"`, `"secondary"`, `"ultimate"`) of the equipped
/// Eidolon weapon.
///
/// Rejects wholesale — never partially — when there is no Eidolon weapon
/// equipped, when the character does not know one of the requested Glifi, when a
/// Modificatore or Parola Antica does not match the *selected* gesture's tags,
/// or when the whole Incisione would exceed the weapon's Capacità Runica. This
/// is `validate_weapon_inscriptions` doing the work, exactly as in
/// `handle_update_inscription_requests`; the only thing that moved is where the
/// registries come from.
#[reducer]
pub fn set_inscription(
    ctx: &ReducerContext,
    slot: String,
    essence: Option<String>,
    modifiers: Vec<String>,
    ancient_word: Option<String>,
) -> Result<(), String> {
    let identity = ctx.sender();
    let target = parse_ability_slot(&slot)?;
    let mut equipment = load_equipment(ctx, identity)?;

    let weapon = equipment
        .get(EquipSlot::Weapon)
        .clone()
        .ok_or_else(|| "no weapon equipped".to_string())?;
    let item = item_registry()
        .get(&weapon.item_id)
        .ok_or_else(|| format!("unknown item {:?}", weapon.item_id.as_str()))?;
    let (Some(abilities), Some(profile)) = (item.ability_loadout(), item.rune_profile()) else {
        return Err(format!(
            "{:?} is not an Eidolon weapon and cannot be inscribed",
            weapon.item_id.as_str()
        ));
    };

    let candidate_slot = Inscription {
        essence: essence.map(EssenceId::new),
        modifiers: modifiers.into_iter().map(ModifierId::new).collect(),
        ancient_word: ancient_word.map(AncientWordId::new),
    };

    // The Vocabolario is per character and survives losing the weapon, so it is
    // checked against `known_glyphs`, not against anything on the item.
    let known = load_known_glyphs(ctx, identity)?;
    if !known.fully_knows(&candidate_slot) {
        return Err("that Incisione uses a Glifo you do not know".to_string());
    }

    let mut inscriptions = weapon.inscriptions.clone().unwrap_or_default();
    *inscriptions.get_mut(target) = candidate_slot;

    // Capacità Runica is shared across the three slots, so the whole set has to
    // be validated even though only one slot changed.
    validate_weapon_inscriptions(
        &inscriptions,
        abilities,
        &weapon.ability_selection,
        profile,
        ability_registry(),
        essence_registry(),
        modifier_registry(),
        ancient_word_registry(),
    )
    .map_err(|reason| format!("Incisione rejected: {reason:?}"))?;

    let mut updated = weapon;
    updated.inscriptions = Some(inscriptions);
    *equipment.get_mut(EquipSlot::Weapon) = Some(updated);
    store_equipment(ctx, identity, &equipment);

    // Nothing derived changes: an Incisione alters what a gesture manifests, not
    // the item's passive stat bonuses nor which spells it offers.
    Ok(())
}

/// Writes the new RootWord-based inscription for the equipped weapon.
///
/// This reducer is additive to [`set_inscription`]. It persists only the new
/// `root_inscription` field, so legacy characters can be migrated without
/// rewriting their old data. All derived spell values remain server-owned.
#[reducer]
pub fn set_root_inscription(
    ctx: &ReducerContext,
    root_word: Option<String>,
    primary_words: Vec<String>,
    secondary_words: Vec<String>,
    ultimate_words: Vec<String>,
) -> Result<(), String> {
    let identity = ctx.sender();
    let mut equipment = load_equipment(ctx, identity)?;
    let weapon = equipment
        .get(EquipSlot::Weapon)
        .clone()
        .ok_or_else(|| "no weapon equipped".to_string())?;
    let item = item_registry()
        .get(&weapon.item_id)
        .ok_or_else(|| format!("unknown item {:?}", weapon.item_id.as_str()))?;
    let abilities = item
        .ability_loadout()
        .ok_or_else(|| format!("{:?} has no ability loadout", weapon.item_id.as_str()))?;
    let profile = item
        .rune_profile()
        .ok_or_else(|| format!("{:?} has no rune profile", weapon.item_id.as_str()))?;

    let known = ctx
        .db
        .known_ancient_language()
        .identity()
        .find(&identity)
        .map(|row| known_ancient_language_from_rows(&row.root_words, &row.ancient_words, &row.base_abilities))
        .ok_or_else(|| "ancient language has not been initialized for this character".to_string())?;

    let root_id = root_word.map(RootWordId::new);
    let root_cost = match &root_id {
        Some(id) => {
            if !known.knows_root_word(id) {
                return Err(format!("Root Word {:?} is not known", id.as_str()));
            }
            root_word_registry()
                .get(id)
                .ok_or_else(|| format!("unknown Root Word {:?}", id.as_str()))?
                .metadata()
                .rune_cost
        }
        None => 0,
    };

    let slot_inputs = [
        (AbilitySlot::Primary, primary_words, 2usize),
        (AbilitySlot::Secondary, secondary_words, 2usize),
        (AbilitySlot::Ultimate, ultimate_words, 1usize),
    ];
    let mut total_cost = root_cost;
    let mut slots = [SlotInscription::default(), SlotInscription::default(), SlotInscription::default()];

    for (index, (slot, word_ids, max_words)) in slot_inputs.into_iter().enumerate() {
        if word_ids.len() > max_words {
            return Err(format!("{slot:?} accepts at most {max_words} Ancient Words"));
        }
        let ability_id = resolve_active_ability(slot, abilities, &weapon.ability_selection)
            .ok_or_else(|| format!("no ability offered for {slot:?}"))?;
        let ability = ability_registry()
            .get(ability_id)
            .ok_or_else(|| format!("unknown ability {:?}", ability_id.as_str()))?;
        let mut ids = HashSet::new();
        let mut groups = HashSet::new();
        let mut words = Vec::with_capacity(word_ids.len());

        for word_id in word_ids {
            let id = AncientWordId::new(word_id);
            if !ids.insert(id.clone()) {
                return Err(format!("duplicate Ancient Word {:?}", id.as_str()));
            }
            if !known.knows_ancient_word(&id) {
                return Err(format!("Ancient Word {:?} is not known", id.as_str()));
            }
            let word = ancient_word_registry()
                .get(&id)
                .ok_or_else(|| format!("unknown Ancient Word {:?}", id.as_str()))?;
            let metadata = word.metadata();
            if !metadata.is_compatible_with(ability.tags()) {
                return Err(format!("Ancient Word {:?} is incompatible with {slot:?}", id.as_str()));
            }
            if let Some(group) = metadata.exclusive_group {
                if !groups.insert(group) {
                    return Err(format!("Ancient Word conflict in group {group:?}"));
                }
            }
            total_cost += metadata.rune_cost;
            words.push(SecondaryWord::new(id));
        }
        slots[index] = SlotInscription { secondary_words: words };
    }

    if total_cost > profile.capacity {
        return Err(format!("rune capacity exceeded: {total_cost} / {}", profile.capacity));
    }

    let mut updated = weapon;
    updated.root_inscription = Some(WeaponInscription {
        root_word: root_id,
        primary: slots[0].clone(),
        secondary: slots[1].clone(),
        ultimate: slots[2].clone(),
    });
    *equipment.get_mut(EquipSlot::Weapon) = Some(updated);
    store_equipment(ctx, identity, &equipment);
    Ok(())
}

/// Writes the independent inscription of an equipped armor item.
///
/// Armor deliberately has its own compact shape instead of pretending to have
/// weapon Primary/Secondary/Ultimate slots. The item remains authoritative for
/// the offered abilities; this reducer only persists the Root Word language data.
#[reducer]
pub fn set_armor_inscription(
    ctx: &ReducerContext,
    slot: String,
    root_word: Option<String>,
    secondary_words: Vec<String>,
) -> Result<(), String> {
    let identity = ctx.sender();
    let target = parse_equip_slot(&slot)?;
    if !matches!(target, EquipSlot::Helmet | EquipSlot::Armor | EquipSlot::Shoes) {
        return Err("armor inscriptions are only valid for helmet, armor or shoes".to_string());
    }

    let mut equipment = load_equipment(ctx, identity)?;
    let item_instance = equipment
        .get(target)
        .clone()
        .ok_or_else(|| format!("equipment slot {slot:?} is empty"))?;
    let item = item_registry()
        .get(&item_instance.item_id)
        .ok_or_else(|| format!("unknown item {:?}", item_instance.item_id.as_str()))?;
    let abilities = item
        .ability_loadout()
        .ok_or_else(|| format!("{:?} has no armor abilities", item_instance.item_id.as_str()))?;
    let profile = item
        .rune_profile()
        .ok_or_else(|| format!("{:?} has no rune profile", item_instance.item_id.as_str()))?;

    let language_row = ctx
        .db
        .known_ancient_language()
        .identity()
        .find(&identity)
        .ok_or_else(|| "ancient language has not been initialized".to_string())?;
    let language = known_ancient_language_from_rows(
        &language_row.root_words,
        &language_row.ancient_words,
        &language_row.base_abilities,
    );

    let root_id = root_word.map(RootWordId::new);
    let mut total_cost = match &root_id {
        Some(id) => {
            if !language.knows_root_word(id) {
                return Err(format!("Root Word {:?} is not known", id.as_str()));
            }
            root_word_registry()
                .get(id)
                .ok_or_else(|| format!("unknown Root Word {:?}", id.as_str()))?
                .metadata()
                .rune_cost
        }
        None => 0,
    };

    if secondary_words.len() > 2 {
        return Err("armor accepts at most 2 Ancient Words".to_string());
    }
    let ability_id = abilities
        .primary
        .first()
        .ok_or_else(|| "armor has no primary ability".to_string())?;
    let ability = ability_registry()
        .get(ability_id)
        .ok_or_else(|| format!("unknown armor ability {:?}", ability_id.as_str()))?;
    let mut seen = HashSet::new();
    let mut words = Vec::with_capacity(secondary_words.len());
    for word_id in secondary_words {
        let id = AncientWordId::new(word_id);
        if !seen.insert(id.clone()) {
            return Err(format!("duplicate Ancient Word {:?}", id.as_str()));
        }
        if !language.knows_ancient_word(&id) {
            return Err(format!("Ancient Word {:?} is not known", id.as_str()));
        }
        let word = ancient_word_registry()
            .get(&id)
            .ok_or_else(|| format!("unknown Ancient Word {:?}", id.as_str()))?;
        let metadata = word.metadata();
        if !metadata.is_compatible_with(ability.tags()) {
            return Err(format!("Ancient Word {:?} is incompatible with armor", id.as_str()));
        }
        total_cost += metadata.rune_cost;
        words.push(SecondaryWord::new(id));
    }

    if total_cost > profile.capacity {
        return Err(format!("rune capacity exceeded: {total_cost} / {}", profile.capacity));
    }

    let mut updated = item_instance;
    updated.armor_inscription = Some(ArmorInscription {
        root_word: root_id,
        secondary_words: words,
    });
    *equipment.get_mut(target) = Some(updated);
    store_equipment(ctx, identity, &equipment);
    Ok(())
}

/// Picks which of the equipped weapon's offered gestures is active on
/// `"primary"` or `"secondary"`.
///
/// The salvage rule is kept: when the new gesture makes the slot's existing
/// Incisione invalid — a Modificatore that needed a tag the old gesture had —
/// the slot's glyphs are cleared rather than the request refused, otherwise a
/// player could get stuck unable to switch gesture at all.
#[reducer]
pub fn set_ability_selection(
    ctx: &ReducerContext,
    slot: String,
    ability_id: String,
) -> Result<(), String> {
    let identity = ctx.sender();
    let target = parse_ability_slot(&slot)?;
    let mut equipment = load_equipment(ctx, identity)?;

    let weapon = equipment
        .get(EquipSlot::Weapon)
        .clone()
        .ok_or_else(|| "no weapon equipped".to_string())?;
    let item = item_registry()
        .get(&weapon.item_id)
        .ok_or_else(|| format!("unknown item {:?}", weapon.item_id.as_str()))?;
    let (Some(abilities), Some(profile)) = (item.ability_loadout(), item.rune_profile()) else {
        return Err(format!(
            "{:?} offers no gestures to choose from",
            weapon.item_id.as_str()
        ));
    };

    let requested = AbilityId::new(ability_id.clone());
    if !abilities.options_for(target).contains(&requested) {
        return Err(format!(
            "{ability_id:?} is not offered on {slot:?} by {:?}",
            weapon.item_id.as_str()
        ));
    }

    let mut selection = weapon.ability_selection.clone();
    selection.assign(target, Some(requested));

    let mut inscriptions = weapon.inscriptions.clone().unwrap_or_default();
    if validate_weapon_inscriptions(
        &inscriptions,
        abilities,
        &selection,
        profile,
        ability_registry(),
        essence_registry(),
        modifier_registry(),
        ancient_word_registry(),
    )
    .is_err()
    {
        *inscriptions.get_mut(target) = Inscription::default();
    }

    let mut updated = weapon;
    updated.ability_selection = selection;
    updated.inscriptions = Some(inscriptions);
    *equipment.get_mut(EquipSlot::Weapon) = Some(updated);
    store_equipment(ctx, identity, &equipment);
    Ok(())
}

// ---------------------------------------------------------------------------
// Shared API: derived state
// ---------------------------------------------------------------------------

/// Recomputes `entity_stats` for `identity` from `player_stats` plus the
/// bonuses of everything currently equipped.
///
/// **Shared API — call this from any reducer that changes a character's base
/// stats or their equipment.** It replaces `bonuses::recompute_equipment_bonuses`,
/// which ran reactively on `Changed<Equipment>`; there is no change detection in
/// a module, so the caller is responsible for invoking it. It is idempotent:
/// running it twice in a row produces the same row, because it always rebuilds
/// from the base values rather than adjusting the previous result. That is what
/// makes the `AppliedEquipmentBonus` snapshot the Bevy server carried around
/// unnecessary — bonuses are never "reverted" here, they are simply not part of
/// what is stored.
///
/// Live pools are preserved: equipment changes `max_health`/`max_mana`, so
/// `current_health` and `current_mana` are taken from the existing
/// `entity_stats` row (falling back to the base row on first computation) and
/// re-clamped to the new maxima. Taking them from `player_stats` instead would
/// silently full-heal a character every time they swapped a helmet.
pub fn recompute_effective_stats(ctx: &ReducerContext, identity: Identity) -> Result<(), String> {
    let player = ctx
        .db
        .player()
        .identity()
        .find(&identity)
        .ok_or_else(|| "no character for this identity".to_string())?;

    // Delegates rather than deriving the stats here. `sim::combat` owns
    // `entity_stats`: it folds equipment *and* timed modifiers into the base,
    // and it is the only writer of `game_entity.speed`. Computing equipment
    // bonuses separately here would produce a row missing every active buff,
    // which the next combat tick would silently overwrite.
    crate::sim::combat::recalculate_effective_stats(ctx, player.entity_id);
    Ok(())
}

/// Clears any hotbar selection the current equipment no longer offers.
///
/// The port of `available_spells::recompute_available_spells`. Same trigger
/// point as the Bevy version — every equipment change — and the same reason:
/// unequipping the item that granted a spell must not leave the key bound to
/// something the character can no longer cast.
///
/// Silent when the character has no hotbar row: that is a state `join` does not
/// produce, and failing an otherwise valid equip over it would be worse than
/// ignoring it.
fn prune_hotbar_to_available(ctx: &ReducerContext, identity: Identity, equipment: &Equipment) {
    let Some(row) = ctx.db.hotbar().identity().find(&identity) else {
        return;
    };
    let choices = available_choices(equipment);
    let mut bar = SpellHotbar::from(&row.slots);

    let mut changed = false;
    for key in [HotbarSlot::Q, HotbarSlot::W, HotbarSlot::E] {
        let stale = bar
            .spell_for_slot(key)
            .is_some_and(|selected| !choices.contains(key, selected));
        if stale {
            log::info!("clearing {key:?} selection: no longer offered by equipped items");
            bar.assign(key, None);
            changed = true;
        }
    }

    if changed {
        ctx.db.hotbar().identity().update(Hotbar {
            identity,
            slots: HotbarRow::from(&bar),
        });
    }
}

/// Which spells the equipped items currently offer for Q/W/E.
fn available_choices(equipment: &Equipment) -> AvailableSpellChoices {
    compute_available_choices(equipment, item_registry())
}

// ---------------------------------------------------------------------------
// Shared API: minting items
// ---------------------------------------------------------------------------

/// Puts a freshly minted esemplare of `item_id` into the first free inventory
/// slot, returning the slot it landed in.
///
/// **Shared API — this is the only sanctioned way to create an item.** It is a
/// plain function and not a reducer on purpose: a client-callable "give me an
/// item" is an item duplication exploit with extra steps. Loot, quest rewards
/// and starter kits call it from inside their own reducer.
///
/// The `item_id` is checked against the registry: an inventory holding an id
/// nothing can look up is a slot the player can never equip or drop.
pub fn grant_item(ctx: &ReducerContext, identity: Identity, item_id: &str) -> Result<u8, String> {
    let id = ItemId::new(item_id.to_string());
    if !item_registry().contains(&id) {
        return Err(format!("unknown item {item_id:?}"));
    }

    let mut inventory = load_inventory(ctx, identity)?;
    let free = inventory
        .slots
        .iter()
        .position(Option::is_none)
        .ok_or_else(|| "inventory is full".to_string())?;

    let mut instance = ItemInstance::new(id);
    instance.instance_id = ItemInstanceId(next_instance_id(ctx));
    inventory.slots[free] = Some(instance);
    store_inventory(ctx, identity, &inventory);
    Ok(free as u8)
}

/// The next free `ItemInstanceId`: one past the highest one stored anywhere.
///
/// `ItemInstance::new` no longer mints an id (it used to be a random `Uuid`, and
/// `getrandom` has no backend in the sandbox), so someone has to. It cannot be
/// an `#[auto_inc]` column either, because instances are not rows — they live
/// *inside* the `Vec<Option<ItemInstanceRow>>` of `inventory` and `equipment`.
///
/// So: scan. The cost is one pass over both tables per minted item, which is
/// fine at the rate items are created and wrong at the rate they would be if
/// this were ever called in a loop. If it becomes hot, the fix is a one-row
/// counter table, which the schema does not currently have.
fn next_instance_id(ctx: &ReducerContext) -> u64 {
    let from_inventories = ctx
        .db
        .inventory()
        .iter()
        .flat_map(|row| row.slots.into_iter().flatten())
        .map(|item| item.instance_id);
    let from_equipment = ctx
        .db
        .equipment()
        .iter()
        .flat_map(|row| row.slots.into_iter().flatten())
        .map(|item| item.instance_id);

    from_inventories.chain(from_equipment).max().unwrap_or(0) + 1
}

// Equipment bonuses are folded into the effective stats by `sim::combat`, which
// also folds in the timed modifiers. Computing them a second time here would
// produce a row missing every active buff.

/// Rejects an item whose equip requirements the character cannot meet.
///
/// `EquipRequirement::MinLevel` is currently unmeetable by construction: the
/// module has no character level anywhere in the schema. Failing closed is the
/// safer half of the trade — no shipped item declares a requirement, so nothing
/// is blocked today, and the day one does it will be blocked loudly instead of
/// silently equipped by a server that forgot to check.
fn check_equip_requirements(requirements: &[EquipRequirement]) -> Result<(), String> {
    for requirement in requirements {
        match requirement {
            EquipRequirement::MinLevel { value: 0 } => {}
            EquipRequirement::MinLevel { value } => {
                return Err(format!(
                    "requires level {value}, and characters have no level yet"
                ))
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Row access
// ---------------------------------------------------------------------------

fn load_inventory(ctx: &ReducerContext, identity: Identity) -> Result<Inventory, String> {
    ctx.db
        .inventory()
        .identity()
        .find(&identity)
        .map(|row| inventory_from_rows(&row.slots))
        .ok_or_else(|| "no character for this identity; call `join` first".to_string())
}

fn store_inventory(ctx: &ReducerContext, identity: Identity, inventory: &Inventory) {
    ctx.db.inventory().identity().update(InventoryTable {
        identity,
        slots: inventory_to_rows(inventory),
    });
}

fn load_equipment(ctx: &ReducerContext, identity: Identity) -> Result<Equipment, String> {
    ctx.db
        .equipment()
        .identity()
        .find(&identity)
        .map(|row| equipment_from_rows(&row.slots))
        .ok_or_else(|| "no character for this identity; call `join` first".to_string())
}

fn store_equipment(ctx: &ReducerContext, identity: Identity, equipment: &Equipment) {
    ctx.db.equipment().identity().update(EquipmentTable {
        identity,
        slots: equipment_to_rows(equipment),
    });
}

fn load_known_glyphs(
    ctx: &ReducerContext,
    identity: Identity,
) -> Result<bevymmo_domain::abilities::KnownGlyphs, String> {
    ctx.db
        .known_glyphs()
        .identity()
        .find(&identity)
        .map(|row| known_glyphs_from_rows(&row.essences, &row.modifiers, &row.ancient_words))
        .ok_or_else(|| "no character for this identity; call `join` first".to_string())
}

// ---------------------------------------------------------------------------
// Parsing the string parameters
// ---------------------------------------------------------------------------
//
// Reducer parameters could be SATS enums instead of strings, and for an enum
// with ten variants that would be tempting. They are strings because the module
// is also driven by hand — `spacetime call bevymmo unequip_item '["weapon"]'` —
// and because the client bindings turn either into the same thing. The parse is
// strict and total, so nothing is lost on the validation side.

/// Parses an equipment slot name, case-insensitively.
fn parse_equip_slot(name: &str) -> Result<EquipSlot, String> {
    EquipSlot::ALL
        .into_iter()
        .find(|slot| slot.label().eq_ignore_ascii_case(name.trim()))
        .ok_or_else(|| {
            format!(
                "unknown equipment slot {name:?}; expected one of bag, helmet, cape, weapon, \
                 armor, offhand, potion, shoes, food, mount"
            )
        })
}

/// Parses a hotbar key name, case-insensitively.
fn parse_hotbar_slot(name: &str) -> Result<HotbarSlot, String> {
    match name.trim().to_ascii_lowercase().as_str() {
        "q" => Ok(HotbarSlot::Q),
        "w" => Ok(HotbarSlot::W),
        "e" => Ok(HotbarSlot::E),
        other => Err(format!("unknown hotbar slot {other:?}; expected q, w or e")),
    }
}

/// Parses an ability slot name, case-insensitively.
fn parse_ability_slot(name: &str) -> Result<AbilitySlot, String> {
    match name.trim().to_ascii_lowercase().as_str() {
        "primary" => Ok(AbilitySlot::Primary),
        "secondary" => Ok(AbilitySlot::Secondary),
        "ultimate" => Ok(AbilitySlot::Ultimate),
        other => Err(format!(
            "unknown ability slot {other:?}; expected primary, secondary or ultimate"
        )),
    }
}
