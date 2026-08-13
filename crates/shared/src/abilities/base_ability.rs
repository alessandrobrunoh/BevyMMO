//! `BaseAbility` — il "gesto" fisico di un'arma. Statico, mai creato dai
//! Glifi (principio cardine del sistema: l'equipaggiamento determina COME
//! un'abilità viene eseguita, l'Incisione determina COSA manifesta).
//!
//! A differenza di `Essence`/`Modifier`/`AncientWord`, una `BaseAbility` è
//! puro dato (nessuna logica che varia da istanza a istanza al di fuori di
//! `default_manifestation`, che ha un default generico derivato dalla
//! geometria) — per questo `#[base_ability(...)]` può generare l'intero
//! `impl BaseAbility for X` senza bisogno di un trait di comportamento
//! separato, a differenza delle altre tre macro gemelle.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::spells::context::{AoeEffect, AoeTargeting, SpellCastContext};

/// Tag invisibili che determinano quali Modificatori/Parole Antiche
/// un'abilità può accettare (es. "Espandere richiede Area").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbilityTag {
    Melee,
    Ranged,
    Projectile,
    Area,
    Ground,
    SingleTarget,
    SelfTarget,
    RepeatCompatible,
    PersistentCompatible,
    EchoCompatible,
}

/// Forma geometrica dell'impatto — condivisa fra il calcolo dell'effetto e
/// il visual (`impact_vfx` disegna esattamente questa forma).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AbilityGeometry {
    Cone { radius: f32, angle_deg: f32 },
    Circle { radius: f32 },
    Projectile { range: f32, speed: f32 },
    SelfBuff { duration_seconds: f32 },
}

/// Parametri numerici, prima o dopo l'applicazione dei Modificatori
/// (§14-24 del design: Espandere/Concentrare/Accelerare/Amplificare/
/// Prolungare agiscono su questi campi).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AbilityParams {
    pub power: f32,
    pub area: f32,
    pub range: f32,
    pub cast_time: f32,
    pub cooldown: f32,
    pub energy_cost: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbilityId(Cow<'static, str>);

impl AbilityId {
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&'static str> for AbilityId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

pub trait BaseAbility: Send + Sync + 'static {
    fn id(&self) -> AbilityId;
    fn display_name(&self) -> &'static str;
    fn tags(&self) -> &'static [AbilityTag];
    fn geometry(&self) -> AbilityGeometry;
    fn base_params(&self) -> AbilityParams;
    /// Clip di animazione del personaggio — SEMPRE la stessa qualunque sia
    /// l'Essenza incisa (il gesto appartiene all'arma, non al Glifo).
    fn animation(&self) -> &'static str;
    /// Forma visiva dell'impatto (particellare/mesh), a cui l'Essenza
    /// applica solo colore/tema (vedi `EssenceVisualTheme`).
    fn impact_vfx(&self) -> &'static str;

    fn has_tag(&self, tag: AbilityTag) -> bool {
        self.tags().contains(&tag)
    }

    /// Emette danno a `power` nella forma dettata dalla geometria di questa
    /// abilità (AoE per Cone/Circle, colpo diretto per Projectile, niente
    /// per SelfBuff). Helper condiviso: qualunque Essenza offensiva lo
    /// riusa per non duplicare il dispatch-per-geometria, cambiando solo
    /// `power` (es. Fuoco lo amplifica) — vedi `essences_impl/fuoco.rs`.
    fn emit_damage_for_geometry(&self, power: f32, params: &AbilityParams, ctx: &mut SpellCastContext) {
        match self.geometry() {
            AbilityGeometry::Cone { radius, .. } | AbilityGeometry::Circle { radius } => {
                let center = ctx.effective_center();
                let area = params.area.max(radius);
                ctx.emit_aoe(
                    center,
                    area,
                    0.0,
                    self.id().as_str().to_string(),
                    AoeEffect::Damage { amount: power, targeting: AoeTargeting::ExcludeCaster },
                );
            }
            AbilityGeometry::Projectile { .. } => {
                if let Some(target) = ctx.target_entity {
                    ctx.emit_damage(target, power);
                }
            }
            AbilityGeometry::SelfBuff { .. } => {
                // Nessun effetto fisico di default: un self-buff senza
                // Essenza non fa nulla, coerente col principio che l'Essenza
                // è ciò che "manifesta" davvero qualcosa.
            }
        }
    }

    /// Manifestazione usata quando lo slot non ha nessuna Essenza incisa
    /// (o quando quella incisa non è conosciuta). Ha un default generico
    /// derivato dalla geometria, per questo NON serve scrivere logica a
    /// mano per ogni `BaseAbility`.
    fn default_manifestation(&self, params: &AbilityParams, ctx: &mut SpellCastContext) {
        self.emit_damage_for_geometry(params.power, params, ctx);
    }
}

pub type ArcBaseAbility = Arc<dyn BaseAbility>;

#[derive(Resource, Default)]
pub struct BaseAbilityRegistry {
    abilities: HashMap<AbilityId, ArcBaseAbility>,
}

impl BaseAbilityRegistry {
    pub fn register(&mut self, ability: ArcBaseAbility) {
        self.abilities.insert(ability.id(), ability);
    }
    pub fn get(&self, id: &AbilityId) -> Option<ArcBaseAbility> {
        self.abilities.get(id).cloned()
    }
    pub fn contains(&self, id: &AbilityId) -> bool {
        self.abilities.contains_key(id)
    }
    pub fn len(&self) -> usize {
        self.abilities.len()
    }
    pub fn is_empty(&self) -> bool {
        self.abilities.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyAbility;
    impl BaseAbility for DummyAbility {
        fn id(&self) -> AbilityId {
            AbilityId::new("dummy")
        }
        fn display_name(&self) -> &'static str {
            "Dummy"
        }
        fn tags(&self) -> &'static [AbilityTag] {
            &[AbilityTag::Melee, AbilityTag::Area]
        }
        fn geometry(&self) -> AbilityGeometry {
            AbilityGeometry::Circle { radius: 3.0 }
        }
        fn base_params(&self) -> AbilityParams {
            AbilityParams { power: 100.0, area: 3.0, range: 0.0, cast_time: 0.5, cooldown: 5.0, energy_cost: 10.0 }
        }
        fn animation(&self) -> &'static str {
            "dummy_anim"
        }
        fn impact_vfx(&self) -> &'static str {
            "dummy_vfx"
        }
    }

    #[test]
    fn has_tag_reads_the_static_tag_list() {
        let ability = DummyAbility;
        assert!(ability.has_tag(AbilityTag::Area));
        assert!(!ability.has_tag(AbilityTag::Projectile));
    }

    #[test]
    fn register_and_lookup_by_id() {
        let mut registry = BaseAbilityRegistry::default();
        registry.register(Arc::new(DummyAbility));
        assert!(registry.contains(&AbilityId::new("dummy")));
        assert_eq!(registry.len(), 1);
    }
}
