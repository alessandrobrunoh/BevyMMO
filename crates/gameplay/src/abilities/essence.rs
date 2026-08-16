//! `Essence` — "cosa manifesta" un'abilità (Vita, Fuoco, Gelo...).
//!
//! Split in due trait, come `Modifier`/`AncientWord`: `Essence` è la parte
//! che `#[essence(...)]` genera per intero da letterali (id, costo,
//! targeting di default, tema visivo); `EssenceEffect::manifest` è la
//! logica vera e propria (chiama `SpellCastContext::emit_*`), che l'autore
//! scrive a mano perché varia troppo da Essenza a Essenza per essere dato
//! puro. La macro genera `Essence::manifest` come un semplice delegate a
//! `EssenceEffect::manifest` — se dimentichi di implementare `EssenceEffect`
//! il compilatore lo dice chiaramente ("trait bound not satisfied"), niente
//! errori misteriosi.

use crate::math::Rgba;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::base_ability::{AbilityParams, BaseAbility};
use crate::spells::context::{AoeTargeting, SpellCastContext};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EssenceVisualTheme {
    pub color: Rgba,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EssenceId(Cow<'static, str>);

impl EssenceId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for EssenceId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

/// Logica di manifestazione — SOLO questa è scritta a mano dall'autore
/// dell'Essenza. Tutto il resto (`Essence`) è generato dalla macro.
pub trait EssenceEffect: Send + Sync + 'static {
    fn manifest(&self, ability: &dyn BaseAbility, params: &AbilityParams, ctx: &mut SpellCastContext);
}

pub trait Essence: Send + Sync + 'static {
    fn id(&self) -> EssenceId;
    fn display_name(&self) -> &'static str;
    fn rune_cost(&self) -> u32;
    /// Regola naturale del bersaglio, senza bisogno di un Glifo "CHI"
    /// dedicato (Vita → alleati, Fuoco → nemici, ecc.).
    fn default_targeting(&self) -> AoeTargeting;
    fn visual_theme(&self) -> EssenceVisualTheme;
    fn manifest(&self, ability: &dyn BaseAbility, params: &AbilityParams, ctx: &mut SpellCastContext);
}

pub type ArcEssence = Arc<dyn Essence>;

#[cfg_attr(feature = "bevy", derive(bevy_ecs::resource::Resource))]
#[derive(Default)]
pub struct EssenceRegistry {
    essences: HashMap<EssenceId, ArcEssence>,
}

impl EssenceRegistry {
    pub fn register(&mut self, essence: ArcEssence) {
        self.essences.insert(essence.id(), essence);
    }
    pub fn get(&self, id: &EssenceId) -> Option<ArcEssence> {
        self.essences.get(id).cloned()
    }
    pub fn contains(&self, id: &EssenceId) -> bool {
        self.essences.contains_key(id)
    }
    pub fn len(&self) -> usize {
        self.essences.len()
    }
    pub fn is_empty(&self) -> bool {
        self.essences.is_empty()
    }
}
