//! `AncientWord` — "quale regola viene modificata" (Eco, Dividere...).
//! Stesso split di `Essence`/`Modifier`: metadata generato dalla macro,
//! `post_process` delegato a `AncientWordEffect` scritto a mano. Gira DOPO
//! la manifestazione dell'Essenza (es. Eco pianifica una seconda emissione).
//!
//! ## Metadata layer
//!
//! Oltre ai metodi ereditati dalla macro (`id`, `display_name`, `required_tag`,
//! `rune_cost`, `post_process`), il trait espone [`AncientWordMetadata`] tramite il
//! metodo [`AncientWord::metadata()`]. L'implementazione di default costruisce i
//! metadati a partire dai metodi legacy, così il codice generato da `#[ancient_word]`
//! continua a compilare senza modifiche.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::base_ability::{AbilityParams, AbilityTag, BaseAbility};
use crate::spells::context::SpellCastContext;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AncientWordId(Cow<'static, str>);

impl AncientWordId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for AncientWordId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

pub trait AncientWordEffect: Send + Sync + 'static {
    fn post_process(&self, ability: &dyn BaseAbility, params: &AbilityParams, ctx: &mut SpellCastContext);
}

/// Metadati statici di un'Ancient Word.
///
/// Incapsula tutti i dati dichiarativi di una Parola Antica: nome, costo in rune,
/// vincoli di compatibilità (tag richiesti/vietati), gruppo di mutua esclusione,
/// fase di risoluzione e priorità visuale.
///
/// Costruito via [`AncientWordMetadata::from_legacy`] per compatibilità con le
/// macro esistenti, o manualmente per contenuto che sfrutta i campi estesi.
#[derive(Debug, Clone)]
pub struct AncientWordMetadata {
    pub display_name: &'static str,
    /// Tag che la `BaseAbility` deve possedere. Singular nella macro, plural qui.
    pub required_tags: Vec<AbilityTag>,
    /// Tag che impediscono l'incisione di questa Parola Antica.
    pub forbidden_tags: Vec<AbilityTag>,
    /// Gruppo di mutua esclusione (es. "echo" — solo una Parola Antica per gruppo).
    pub exclusive_group: Option<&'static str>,
    /// Fase di risoluzione durante la composizione della spell (minore = prima).
    pub phase: u8,
    /// Priorità visuale; più alto = disegnato sopra gli altri (z-index logico).
    pub visual_priority: i32,
    pub rune_cost: u32,
}

impl AncientWordMetadata {
    /// Costruisce metadati partendo dai campi singoli della vecchia interfaccia.
    /// Usato dall'implementazione di default di [`AncientWord::metadata()`].
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

    /// Controlla se un'abilità con i tag dati è compatibile con questa Parola Antica.
    #[inline]
    pub fn is_compatible_with(&self, tags: &[AbilityTag]) -> bool {
        self.required_tags.iter().any(|t| tags.contains(t))
            && !self.forbidden_tags.iter().any(|t| tags.contains(t))
    }
}

pub trait AncientWord: Send + Sync + 'static {
    fn id(&self) -> AncientWordId;
    fn display_name(&self) -> &'static str;
    fn required_tag(&self) -> AbilityTag;
    fn rune_cost(&self) -> u32;
    fn post_process(&self, ability: &dyn BaseAbility, params: &AbilityParams, ctx: &mut SpellCastContext);

    /// Metadati statici completi di questa Parola Antica.
    ///
    /// L'implementazione di default costruisce [`AncientWordMetadata`] a partire
    /// dai metodi legacy (`display_name`, `required_tag`, `rune_cost`).
    /// Le implementazioni possono sovrascrivere questo metodo per fornire
    /// `forbidden_tags`, `exclusive_group`, `phase` o `visual_priority`.
    fn metadata(&self) -> AncientWordMetadata {
        AncientWordMetadata::from_legacy(self.display_name(), self.required_tag(), self.rune_cost())
    }
}

pub type ArcAncientWord = Arc<dyn AncientWord>;

#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
#[derive(Default)]
pub struct AncientWordRegistry {
    words: HashMap<AncientWordId, ArcAncientWord>,
}

impl AncientWordRegistry {
    pub fn register(&mut self, word: ArcAncientWord) {
        self.words.insert(word.id(), word);
    }
    pub fn get(&self, id: &AncientWordId) -> Option<ArcAncientWord> {
        self.words.get(id).cloned()
    }
    pub fn contains(&self, id: &AncientWordId) -> bool {
        self.words.contains_key(id)
    }
    pub fn len(&self) -> usize {
        self.words.len()
    }
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyAncientWord;

    impl AncientWordEffect for DummyAncientWord {
        fn post_process(
            &self,
            _ability: &dyn BaseAbility,
            _params: &AbilityParams,
            _ctx: &mut SpellCastContext,
        ) {
        }
    }

    impl AncientWord for DummyAncientWord {
        fn id(&self) -> AncientWordId {
            AncientWordId::new("dummy_ancient")
        }
        fn display_name(&self) -> &'static str {
            "Dummy Ancient"
        }
        fn required_tag(&self) -> AbilityTag {
            AbilityTag::Area
        }
        fn rune_cost(&self) -> u32 {
            3
        }
        fn post_process(
            &self,
            ability: &dyn BaseAbility,
            params: &AbilityParams,
            ctx: &mut SpellCastContext,
        ) {
            <Self as AncientWordEffect>::post_process(self, ability, params, ctx)
        }
    }

    #[test]
    fn metadata_from_legacy_maps_singular_required_tag() {
        let word = DummyAncientWord;
        let meta = word.metadata();

        assert_eq!(meta.display_name, "Dummy Ancient");
        assert_eq!(meta.required_tags, vec![AbilityTag::Area]);
        assert_eq!(meta.rune_cost, 3);
        assert!(meta.forbidden_tags.is_empty());
        assert_eq!(meta.exclusive_group, None);
        assert_eq!(meta.phase, 0);
        assert_eq!(meta.visual_priority, 0);
    }

    #[test]
    fn metadata_default_matches_legacy_accessors() {
        let word = DummyAncientWord;
        let meta = word.metadata();

        // The default implementation must mirror the legacy methods exactly.
        assert_eq!(meta.display_name, word.display_name());
        assert_eq!(meta.required_tags, vec![word.required_tag()]);
        assert_eq!(meta.rune_cost, word.rune_cost());
    }

    #[test]
    fn is_compatible_with_required_tag_present() {
        let meta = AncientWordMetadata::from_legacy("Test", AbilityTag::Projectile, 2);
        assert!(meta.is_compatible_with(&[AbilityTag::Projectile, AbilityTag::Ranged]));
    }

    #[test]
    fn is_compatible_with_required_tag_missing() {
        let meta = AncientWordMetadata::from_legacy("Test", AbilityTag::Area, 2);
        assert!(!meta.is_compatible_with(&[AbilityTag::Projectile, AbilityTag::Ranged]));
    }

    #[test]
    fn is_compatible_with_forbidden_tag_blocks() {
        let mut meta = AncientWordMetadata::from_legacy("Test", AbilityTag::Area, 2);
        meta.forbidden_tags.push(AbilityTag::PersistentCompatible);
        assert!(!meta.is_compatible_with(&[AbilityTag::Area, AbilityTag::PersistentCompatible]));
    }

    #[test]
    fn is_compatible_with_forbidden_tag_absent_is_ok() {
        let mut meta = AncientWordMetadata::from_legacy("Test", AbilityTag::Area, 2);
        meta.forbidden_tags.push(AbilityTag::SelfTarget);
        assert!(meta.is_compatible_with(&[AbilityTag::Area]));
    }

    #[test]
    fn rich_metadata_all_fields() {
        let meta = AncientWordMetadata {
            display_name: "Echo Word",
            required_tags: vec![AbilityTag::Ranged, AbilityTag::Projectile],
            forbidden_tags: vec![AbilityTag::PersistentCompatible],
            exclusive_group: Some("echo"),
            phase: 2,
            visual_priority: 10,
            rune_cost: 5,
        };

        assert_eq!(meta.display_name, "Echo Word");
        assert_eq!(meta.required_tags.len(), 2);
        assert_eq!(meta.forbidden_tags.len(), 1);
        assert_eq!(meta.exclusive_group, Some("echo"));
        assert_eq!(meta.phase, 2);
        assert_eq!(meta.visual_priority, 10);
        assert_eq!(meta.rune_cost, 5);
    }

    #[test]
    fn registry_register_and_retrieve() {
        let mut reg = AncientWordRegistry::default();
        let word: ArcAncientWord = Arc::new(DummyAncientWord);
        reg.register(word.clone());

        assert!(reg.contains(&AncientWordId::new("dummy_ancient")));
        let retrieved = reg.get(&AncientWordId::new("dummy_ancient")).unwrap();
        assert_eq!(retrieved.display_name(), "Dummy Ancient");
        // Metadata works through trait object.
        assert_eq!(retrieved.metadata().rune_cost, 3);
    }
}
