//! `RootWord` — parola radice che definisce l'identità fondamentale di
//! un'abilità prima della trasformazione in blueprint. È il primo strato
//! di personalizzazione: la Root Word sceglie "cosa" l'abilità è a livello
//! profondo (Danno, Cura, Utilità...), mentre le Essence/Ancient Words
//! modificano "come" si manifesta.
//!
//! Stessa architettura di `Essence`/`AncientWord`: metadata statici +
//! effect hook scritto a mano, registry centralizzato.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::base_ability::AbilityParams;
use super::AbilityBlueprint;

/// Identificatore unico di una Root Word.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RootWordId(Cow<'static, str>);

impl RootWordId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for RootWordId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

/// Metadati statici di una Root Word — descrizione leggibile e costo.
#[derive(Debug, Clone)]
pub struct RootWordMetadata {
    pub display_name: &'static str,
    pub description: &'static str,
    pub rune_cost: u32,
}

/// Effect hook: trasforma il blueprint base applicando la semantica della
/// Root Word. Viene chiamato durante la costruzione del blueprint finale,
/// **prima** che Essence e AncientWords post-processino il risultato.
///
/// Implementazioni tipiche:
/// - `DamageRootWord`: imposta tags di danno, calcola scaling
/// - `HealRootWord`: sostituisce tag con cura, inverte targeting nemico → alleato
/// - `UtilityRootWord`: aggiunge tag di crowd-control o movimento
pub trait RootWordEffect: Send + Sync + 'static {
    fn apply_to_blueprint(&self, blueprint: &mut AbilityBlueprint, params: &AbilityParams);
}

/// Trait completo per una Root Word: metadata + effect hook.
pub trait RootWord: Send + Sync + 'static {
    /// Identificatore unico.
    fn id(&self) -> RootWordId;

    /// Metadati statici (nome, descrizione, costo).
    fn metadata(&self) -> &RootWordMetadata;

    /// Effect hook: modifica il blueprint in base alla semantica di questa Root Word.
    fn apply_to_blueprint(&self, blueprint: &mut AbilityBlueprint, params: &AbilityParams);
}

/// Versione arcata per lo sharing nel registry.
pub type ArcRootWord = Arc<dyn RootWord>;

/// Registry centrale di tutte le Root Words registrate.
#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
#[derive(Default)]
pub struct RootWordRegistry {
    words: HashMap<RootWordId, ArcRootWord>,
}

impl RootWordRegistry {
    /// Registra una nuova Root Word. Se l'id esiste già, sovrascrive.
    pub fn register(&mut self, word: ArcRootWord) {
        self.words.insert(word.id(), word);
    }

    /// Recupera una Root Word per id. Restituisce `None` se non esiste.
    pub fn get(&self, id: &RootWordId) -> Option<ArcRootWord> {
        self.words.get(id).cloned()
    }

    /// Verifica se una Root Word è registrata.
    pub fn contains(&self, id: &RootWordId) -> bool {
        self.words.contains_key(id)
    }

    /// Numero di Root Words registrate.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// Vero se non ci sono Root Words registrate.
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_word_id_from_str() {
        let id = RootWordId::from("damage");
        assert_eq!(id.as_str(), "damage");
    }

    #[test]
    fn root_word_id_equality() {
        let a = RootWordId::new("heal");
        let b = RootWordId::from("heal");
        assert_eq!(a, b);
    }

    #[test]
    fn root_word_metadata() {
        let meta = RootWordMetadata {
            display_name: "Danno",
            description: "Infligge danno ai bersagli",
            rune_cost: 1,
        };
        assert_eq!(meta.display_name, "Danno");
        assert_eq!(meta.rune_cost, 1);
    }

    // Dummy implementation per testare il registry
    struct TestRootWord;

    impl RootWord for TestRootWord {
        fn id(&self) -> RootWordId {
            RootWordId::from("test")
        }

        fn metadata(&self) -> &RootWordMetadata {
            static META: RootWordMetadata = RootWordMetadata {
                display_name: "Test",
                description: "Root word di test",
                rune_cost: 0,
            };
            &META
        }

        fn apply_to_blueprint(&self, _blueprint: &mut AbilityBlueprint, _params: &AbilityParams) {
            // noop per test
        }
    }

    #[test]
    fn registry_register_and_get() {
        let mut reg = RootWordRegistry::default();
        let word = Arc::new(TestRootWord);

        assert!(!reg.contains(&RootWordId::from("test")));
        assert_eq!(reg.len(), 0);

        reg.register(word.clone());
        assert!(reg.contains(&RootWordId::from("test")));
        assert_eq!(reg.len(), 1);

        let retrieved = reg.get(&RootWordId::from("test"));
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), RootWordId::from("test"));
    }

    #[test]
    fn registry_overwrite() {
        let mut reg = RootWordRegistry::default();
        reg.register(Arc::new(TestRootWord));
        assert_eq!(reg.len(), 1);

        // Re-register should overwrite (no duplicate keys)
        reg.register(Arc::new(TestRootWord));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn registry_get_missing() {
        let reg = RootWordRegistry::default();
        assert!(reg.get(&RootWordId::from("nonexistent")).is_none());
    }

    #[test]
    fn registry_empty() {
        let reg = RootWordRegistry::default();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }
}
