//! `Modifier` — "come cambia il comportamento" (Persistere, Espandere...).
//! Stesso split di `Essence`: `Modifier` generato dalla macro, `transform`
//! delegato a `ModifierEffect` scritto a mano.

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

pub trait Modifier: Send + Sync + 'static {
    fn id(&self) -> ModifierId;
    fn display_name(&self) -> &'static str;
    /// Tag che la `BaseAbility` deve possedere perché questo Modificatore
    /// sia incidibile (es. Espandere richiede `AbilityTag::Area`).
    fn required_tag(&self) -> AbilityTag;
    fn rune_cost(&self) -> u32;
    fn transform(&self, params: &mut AbilityParams);
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
