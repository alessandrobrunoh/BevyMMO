//! `AncientWord` — "quale regola viene modificata" (Eco, Dividere...).
//! Stesso split di `Essence`/`Modifier`: metadata generato dalla macro,
//! `post_process` delegato a `AncientWordEffect` scritto a mano. Gira DOPO
//! la manifestazione dell'Essenza (es. Eco pianifica una seconda emissione).

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
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

pub trait AncientWord: Send + Sync + 'static {
    fn id(&self) -> AncientWordId;
    fn display_name(&self) -> &'static str;
    fn required_tag(&self) -> AbilityTag;
    fn rune_cost(&self) -> u32;
    fn post_process(&self, ability: &dyn BaseAbility, params: &AbilityParams, ctx: &mut SpellCastContext);
}

pub type ArcAncientWord = Arc<dyn AncientWord>;

#[derive(Resource, Default)]
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
