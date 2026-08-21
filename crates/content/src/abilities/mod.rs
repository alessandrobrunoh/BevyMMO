//! Base-ability content and its registry.

pub mod blade_storm;
pub mod cleave;
pub mod lunge;

use crate::abilities::BaseAbilityRegistry;

/// Builds the registry containing every base ability shipped by this game build.
pub fn default_base_abilities() -> BaseAbilityRegistry {
    let mut registry = BaseAbilityRegistry::default();
    cleave::register(&mut registry);
    lunge::register(&mut registry);
    blade_storm::register(&mut registry);
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_abilities_contains_sword_gestures() {
        let registry = default_base_abilities();
        assert_eq!(registry.len(), 3);
        assert!(registry.contains(&crate::abilities::AbilityId::new("cleave")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("lunge")));
        assert!(registry.contains(&crate::abilities::AbilityId::new("blade_storm")));
    }

    #[test]
    fn ability_icon_filenames_match_ability_ids() {
        let icons =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/abilities/icons");
        let registry = default_base_abilities();
        let Ok(entries) = std::fs::read_dir(&icons) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("icon dir entry").path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("png") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("utf-8 icon filename");
            assert!(
                registry.contains(&crate::abilities::AbilityId::new(stem.to_string())),
                "icon {stem}.png has no matching ability id; the hotbar loads abilities/icons/{{id}}.png"
            );
        }
    }
}
