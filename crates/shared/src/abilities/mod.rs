//! Sistema Eidolon: gesti fissi dell'equipaggiamento (`BaseAbility`) +
//! Glifi incisi dal giocatore (`Essence`/`Modifier`/`AncientWord`) = spell
//! finale. Mirrors `crate::spells`/`crate::items` nello stile (trait +
//! registry + id), con quattro macro gemelle in `bevymmo-props-macro` per
//! ridurre il boilerplate di ogni nuovo pezzo di contenuto.

pub mod aim;
pub mod ancient_word;
pub mod base_ability;
pub mod cooldowns;
pub mod essence;
pub mod events;
pub mod inscription;
pub mod known_glyphs;
pub mod modifier;
pub mod resolve;
pub mod slot;
pub mod weapon_abilities;

pub use aim::AbilityAim;
pub use ancient_word::{AncientWord, AncientWordEffect, AncientWordId, AncientWordRegistry, ArcAncientWord};
pub use cooldowns::AbilityCooldowns;
pub use events::EidolonCastRequest;
pub use base_ability::{AbilityGeometry, AbilityId, AbilityParams, AbilityTag, ArcBaseAbility, BaseAbility, BaseAbilityRegistry};
pub use essence::{ArcEssence, Essence, EssenceEffect, EssenceId, EssenceRegistry, EssenceVisualTheme};
pub use inscription::{validate_weapon_inscriptions, Inscription, InscriptionError, RuneProfile, WeaponInscriptions};
pub use known_glyphs::KnownGlyphs;
pub use modifier::{ArcModifier, Modifier, ModifierEffect, ModifierId, ModifierRegistry};
pub use resolve::{
    cast_inscribed_slot, resolve_ability_params, resolve_slot_preview, CastBlockedReason,
    SlotPreview,
};
pub use slot::AbilitySlot;
pub use weapon_abilities::{resolve_active_ability, AbilitySelection, WeaponAbilities};
