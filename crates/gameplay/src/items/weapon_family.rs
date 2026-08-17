//! Shared metadata for weapon families.
//!
//! A family is a category of weapons with common identity and compatibility
//! rules. Concrete item definitions remain the variants and own their stats
//! and execution-specific behavior.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::abilities::AbilityLoadout;
use crate::registry::Registry;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeaponFamilyId(Cow<'static, str>);

impl WeaponFamilyId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for WeaponFamilyId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponFamilyMetadata {
    pub id: WeaponFamilyId,
    pub display_name: &'static str,
    /// Default ability pools for this family. Items may override with their own.
    pub ability_loadout: Option<AbilityLoadout>,
}

pub trait WeaponFamily: Send + Sync + 'static {
    fn metadata() -> WeaponFamilyMetadata;
}

#[derive(Debug, Default)]
pub struct WeaponFamilyRegistry {
    families: Registry<WeaponFamilyId, WeaponFamilyMetadata>,
}

impl WeaponFamilyRegistry {
    pub fn register(&mut self, metadata: WeaponFamilyMetadata) {
        self.families.insert(metadata.id.clone(), metadata);
    }

    pub fn get(&self, id: &WeaponFamilyId) -> Option<&WeaponFamilyMetadata> {
        self.families.get(id)
    }

    pub fn contains(&self, id: &WeaponFamilyId) -> bool {
        self.families.contains(id)
    }

    pub fn len(&self) -> usize {
        self.families.len()
    }

    pub fn is_empty(&self) -> bool {
        self.families.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_roundtrip_through_str() {
        let id = WeaponFamilyId::new("staff");
        assert_eq!(id.as_str(), "staff");
    }

    #[test]
    fn registry_replaces_and_reads_metadata() {
        let mut registry = WeaponFamilyRegistry::default();
        let id = WeaponFamilyId::new("staff");
        registry.register(WeaponFamilyMetadata {
            id: id.clone(),
            display_name: "Staff",
            ability_loadout: None,
        });

        assert!(registry.contains(&id));
        assert_eq!(
            registry.get(&id).map(|family| family.display_name),
            Some("Staff")
        );
        assert_eq!(registry.len(), 1);
    }
}
