//! `Modifier` — "come cambia il comportamento" (Persistere, Espandere...).
//! Stesso split di `Essence`: `Modifier` generato dalla macro, `transform`
//! delegato a `ModifierEffect` scritto a mano.
//!
//! ## Metadata layer
//!
//! Espone [`ModifierMetadata`] tramite [`Modifier::modifier_metadata()`] con
//! implementazione di default compatibile con la macro `#[modifier(...)]`.
//! Condivide la stessa forma di [`crate::abilities::ancient_word::AncientWordMetadata`]
//! ma rimane un tipo distinto per evitare accoppiamenti non necessari.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::base_ability::{AbilityParams, AbilityTag};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModifierId(Cow<'static, str>);

impl ModifierId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for ModifierId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

pub trait ModifierEffect: Send + Sync + 'static {
    fn transform(&self, params: &mut AbilityParams);
}

/// Metadati statici di un Modifier.
///
/// Forma speculare a [`crate::abilities::ancient_word::AncientWordMetadata`] per
/// mantenere i due sistemi glyph allineati senza forzare un tipo unificato.
#[derive(Debug, Clone)]
pub struct ModifierMetadata {
    pub display_name: &'static str,
    /// Tag che la `BaseAbility` deve possedere.
    pub required_tags: Vec<AbilityTag>,
    /// Tag che impediscono l'incisione di questo Modifier.
    pub forbidden_tags: Vec<AbilityTag>,
    /// Gruppo di mutua esclusione.
    pub exclusive_group: Option<&'static str>,
    /// Fase di risoluzione.
    pub phase: u8,
    /// Priorità visuale.
    pub visual_priority: i32,
    pub rune_cost: u32,
}

impl ModifierMetadata {
    /// Costruisce metadati partendo dai campi singoli della vecchia interfaccia.
    pub fn from_legacy(
        display_name: &'static str,
        required_tag: AbilityTag,
        rune_cost: u32,
    ) -> Self {
        Self {
            display_name,
            required_tags: vec![required_tag],
            forbidden_tags: Vec::new(),
            exclusive_group: None,
            phase: 0,
            visual_priority: 0,
            rune_cost,
        }
    }

    /// Controlla compatibilità con un set di tag.
    #[inline]
    pub fn is_compatible_with(&self, tags: &[AbilityTag]) -> bool {
        self.required_tags.iter().any(|t| tags.contains(t))
            && !self.forbidden_tags.iter().any(|t| tags.contains(t))
    }
}

pub trait Modifier: Send + Sync + 'static {
    fn id(&self) -> ModifierId;
    fn display_name(&self) -> &'static str;
    /// Tag che la `BaseAbility` deve possedere perché questo Modificatore
    /// sia incidibile (es. Espandere richiede `AbilityTag::Area`).
    fn required_tag(&self) -> AbilityTag;
    fn rune_cost(&self) -> u32;
    fn transform(&self, params: &mut AbilityParams);

    /// Metadati statici completi di questo Modifier.
    ///
    /// L'implementazione di default costruisce [`ModifierMetadata`] a partire
    /// dai metodi legacy (`display_name`, `required_tag`, `rune_cost`).
    fn modifier_metadata(&self) -> ModifierMetadata {
        ModifierMetadata::from_legacy(self.display_name(), self.required_tag(), self.rune_cost())
    }
}

pub type ArcModifier = Arc<dyn Modifier>;

#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
#[derive(Default)]
pub struct ModifierRegistry {
    modifiers: HashMap<ModifierId, ArcModifier>,
}

impl ModifierRegistry {
    pub fn register(&mut self, modifier: ArcModifier) {
        self.modifiers.insert(modifier.id(), modifier);
    }
    pub fn get(&self, id: &ModifierId) -> Option<ArcModifier> {
        self.modifiers.get(id).cloned()
    }
    pub fn contains(&self, id: &ModifierId) -> bool {
        self.modifiers.contains_key(id)
    }
    pub fn len(&self) -> usize {
        self.modifiers.len()
    }
    pub fn is_empty(&self) -> bool {
        self.modifiers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyModifier;

    impl ModifierEffect for DummyModifier {
        fn transform(&self, _params: &mut AbilityParams) {}
    }

    impl Modifier for DummyModifier {
        fn id(&self) -> ModifierId {
            ModifierId::new("dummy_mod")
        }
        fn display_name(&self) -> &'static str {
            "Dummy Mod"
        }
        fn required_tag(&self) -> AbilityTag {
            AbilityTag::Area
        }
        fn rune_cost(&self) -> u32 {
            2
        }
        fn transform(&self, params: &mut AbilityParams) {
            <Self as ModifierEffect>::transform(self, params)
        }
    }

    #[test]
    fn metadata_from_legacy_maps_singular_required_tag() {
        let mod_ = DummyModifier;
        let meta = mod_.modifier_metadata();

        assert_eq!(meta.display_name, "Dummy Mod");
        assert_eq!(meta.required_tags, vec![AbilityTag::Area]);
        assert_eq!(meta.rune_cost, 2);
        assert!(meta.forbidden_tags.is_empty());
        assert_eq!(meta.exclusive_group, None);
        assert_eq!(meta.phase, 0);
        assert_eq!(meta.visual_priority, 0);
    }

    #[test]
    fn metadata_default_matches_legacy_accessors() {
        let mod_ = DummyModifier;
        let meta = mod_.modifier_metadata();

        assert_eq!(meta.display_name, mod_.display_name());
        assert_eq!(meta.required_tags, vec![mod_.required_tag()]);
        assert_eq!(meta.rune_cost, mod_.rune_cost());
    }

    #[test]
    fn is_compatible_with_required_tag_present() {
        let meta = ModifierMetadata::from_legacy("Test", AbilityTag::Projectile, 1);
        assert!(meta.is_compatible_with(&[AbilityTag::Projectile]));
    }

    #[test]
    fn is_compatible_with_required_tag_missing() {
        let meta = ModifierMetadata::from_legacy("Test", AbilityTag::Area, 1);
        assert!(!meta.is_compatible_with(&[AbilityTag::Ranged, AbilityTag::SingleTarget]));
    }

    #[test]
    fn is_compatible_with_forbidden_tag_blocks() {
        let mut meta = ModifierMetadata::from_legacy("Test", AbilityTag::Area, 1);
        meta.forbidden_tags.push(AbilityTag::PersistentCompatible);
        assert!(!meta.is_compatible_with(&[AbilityTag::Area, AbilityTag::PersistentCompatible]));
    }

    #[test]
    fn rich_metadata_all_fields() {
        let meta = ModifierMetadata {
            display_name: "Expand",
            required_tags: vec![AbilityTag::Area],
            forbidden_tags: vec![AbilityTag::SingleTarget],
            exclusive_group: Some("shape"),
            phase: 1,
            visual_priority: 5,
            rune_cost: 4,
        };

        assert_eq!(meta.required_tags.len(), 1);
        assert_eq!(meta.forbidden_tags.len(), 1);
        assert_eq!(meta.exclusive_group, Some("shape"));
        assert_eq!(meta.phase, 1);
        assert_eq!(meta.visual_priority, 5);
    }

    #[test]
    fn registry_round_trip() {
        let mut reg = ModifierRegistry::default();
        let m: ArcModifier = Arc::new(DummyModifier);
        reg.register(m.clone());

        assert!(reg.contains(&ModifierId::new("dummy_mod")));
        let retrieved = reg.get(&ModifierId::new("dummy_mod")).unwrap();
        // Legacy accessor works through trait object.
        assert_eq!(retrieved.required_tag(), AbilityTag::Area);
        // New metadata works through trait object.
        assert_eq!(retrieved.modifier_metadata().rune_cost, 2);
    }
}
