//! Root Word Danno — definisce l'identità fondamentale di un'abilità di danno.
//! Imposta i tag di danno e calcola lo scaling base prima che Essence e
//! AncientWords post-processino il risultato.

use bevymmo_props_macro::root_word;

use crate::abilities::{
    AbilityBlueprint, AbilityParams, ManifestationPayload, RootWordEffect, RootWordRegistry,
};

#[root_word(
    id = "damage",
    name = "Danno",
    description = "Infligge danno ai bersagli",
    rune_cost = 1
)]
pub struct DamageRootWord;

/// Adds this content package to the root word registry.
pub fn register(registry: &mut RootWordRegistry) {
    DamageRootWord::register(registry);
}

impl DamageRootWord {
    /// Moltiplicatore base per le abilità di danno.
    pub const BASE_SCALING: f32 = 1.0;
}

impl RootWordEffect for DamageRootWord {
    fn apply_to_blueprint(&self, blueprint: &mut AbilityBlueprint, _params: &AbilityParams) {
        // Tagga il blueprint come abilità di danno (usa tag esistenti)
        blueprint.tags.push(crate::abilities::AbilityTag::Melee);
        blueprint.payload = ManifestationPayload::damage([]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::RootWord;

    #[test]
    fn id_is_stable() {
        assert_eq!(DamageRootWord::ID, "damage");
    }

    #[test]
    fn metadata_values() {
        let word = DamageRootWord;
        let meta = word.metadata();
        assert_eq!(meta.display_name, "Danno");
        assert_eq!(meta.description, "Infligge danno ai bersagli");
        assert_eq!(meta.rune_cost, 1);
    }
}
