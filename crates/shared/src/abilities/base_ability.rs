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

use crate::crowd_control::CrowdControlKind;
use crate::spells::context::{AoeEffect, AoeShape, AoeTargeting, SpellCastContext};

/// Raggio d'impatto di una palla lanciata da un gesto `Projectile`.
pub const PROJECTILE_HIT_RADIUS: f32 = 1.0;

/// Semi-larghezza del "corridoio" davanti al lanciatore entro cui un gesto
/// `Projectile` senza bersaglio selezionato aggancia la prima entità (§ "una
/// palla davanti a sé": si mira guardando, non cliccando).
pub const FORWARD_LANE_HALF_WIDTH: f32 = 1.5;

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

/// Direzione orizzontale normalizzata (l'altezza non conta: si gioca su un
/// piano). `Vec3::ZERO` se la direzione è degenere.
fn flat_direction(direction: Vec3) -> Vec3 {
    Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero()
}

/// Scarto orizzontale da `origin` a `point`.
fn flat_offset(origin: Vec3, point: Vec3) -> Vec3 {
    Vec3::new(point.x - origin.x, 0.0, point.z - origin.z)
}

/// Riporta `target` entro `range` dal lanciatore. `range <= 0.0` = nessun
/// limite (il gesto si piazza dove vuole il giocatore).
fn clamp_to_range(origin: Vec3, target: Vec3, range: f32) -> Vec3 {
    let flat_target = Vec3::new(target.x, 0.0, target.z);
    if range <= 0.0 {
        return flat_target;
    }
    let offset = flat_offset(origin, flat_target);
    let distance = offset.length();
    if distance <= range {
        flat_target
    } else {
        Vec3::new(origin.x, 0.0, origin.z) + offset / distance * range
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

    /// Ritardo fra il lancio e l'impatto. > 0 significa "cerchio di
    /// preavviso a terra, poi lo schianto" (il Meteorite); il client legge
    /// lo stesso valore da qui per far durare il marker esattamente quanto
    /// l'attesa reale. Default: impatto immediato.
    fn impact_delay(&self) -> f32 {
        0.0
    }

    /// Stun applicato all'impatto, in secondi. Default 0.0 = il gesto non ha
    /// componente di controllo, solo danno.
    fn stun_seconds(&self) -> f32 {
        0.0
    }

    fn has_tag(&self, tag: AbilityTag) -> bool {
        self.tags().contains(&tag)
    }

    /// Dove il gesto manifesta il proprio effetto.
    ///
    /// - `Circle`: il punto indicato dal mouse, clampato a `params.range`
    ///   attorno al lanciatore (0.0 = nessun limite di gittata).
    /// - `Cone`: il lanciatore stesso, che è l'APICE del settore — la forma
    ///   vera e propria la porta [`Self::impact_shape`].
    /// - Altro: il lanciatore stesso.
    fn impact_center(&self, params: &AbilityParams, ctx: &SpellCastContext) -> Vec3 {
        match self.geometry() {
            AbilityGeometry::Circle { .. } => {
                clamp_to_range(ctx.caster_position, ctx.effective_center(), params.range)
            }
            AbilityGeometry::Cone { .. }
            | AbilityGeometry::Projectile { .. }
            | AbilityGeometry::SelfBuff { .. } => ctx.caster_position,
        }
    }

    /// Raggio effettivo dell'impatto ad area (0.0 per proiettili/self-buff).
    /// Per il cono è la gittata del settore misurata dall'apice.
    fn impact_radius(&self, params: &AbilityParams) -> f32 {
        match self.geometry() {
            AbilityGeometry::Cone { radius, .. } | AbilityGeometry::Circle { radius } => {
                params.area.max(radius)
            }
            AbilityGeometry::Projectile { .. } | AbilityGeometry::SelfBuff { .. } => 0.0,
        }
    }

    /// Forma coperta attorno a [`Self::impact_center`].
    ///
    /// È il terzo membro della terna centro/raggio/forma: chiunque debba
    /// sapere "quale superficie tocca questo gesto" — il server per applicare
    /// l'effetto, il client per disegnare l'anteprima di mira — la chiede qui
    /// invece di ricostruirla dalla geometria.
    fn impact_shape(&self, ctx: &SpellCastContext) -> AoeShape {
        match self.geometry() {
            AbilityGeometry::Cone { angle_deg, .. } => AoeShape::Cone {
                direction: flat_direction(ctx.caster_look_direction),
                angle_deg,
            },
            AbilityGeometry::Circle { .. }
            | AbilityGeometry::Projectile { .. }
            | AbilityGeometry::SelfBuff { .. } => AoeShape::Circle,
        }
    }

    /// Chi incassa la palla di un gesto `Projectile`.
    ///
    /// Il bersaglio selezionato vince, ma solo se è davvero davanti e a
    /// portata; altrimenti si aggancia la prima entità nel corridoio frontale
    /// (nessuna selezione richiesta — si spara dove si guarda).
    fn projectile_target(&self, ctx: &SpellCastContext) -> Option<Entity> {
        let AbilityGeometry::Projectile { range, .. } = self.geometry() else {
            return None;
        };
        let forward = flat_direction(ctx.caster_look_direction);
        if forward == Vec3::ZERO {
            return ctx.target_entity;
        }

        if let Some(selected) = ctx.target_entity {
            let in_front = ctx
                .potential_targets
                .iter()
                .find(|(entity, _)| *entity == selected)
                .map(|(_, position)| {
                    let offset = flat_offset(ctx.caster_position, *position);
                    let along = offset.dot(forward);
                    along > 0.0 && along <= range
                })
                .unwrap_or(false);
            if in_front {
                return Some(selected);
            }
        }

        ctx.potential_targets
            .iter()
            .filter(|(entity, _)| *entity != ctx.caster)
            .filter_map(|(entity, position)| {
                let offset = flat_offset(ctx.caster_position, *position);
                let along = offset.dot(forward);
                if along <= 0.0 || along > range {
                    return None;
                }
                if (offset - forward * along).length() > FORWARD_LANE_HALF_WIDTH {
                    return None;
                }
                Some((*entity, along))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(entity, _)| entity)
    }

    /// Piazza una regione con centro, raggio, forma e ritardo DI QUESTO gesto.
    ///
    /// Unico punto in cui la geometria di un'abilità si traduce in una
    /// `AoeSpawnRequest`: le Essenze che aggiungono un proprio effetto sopra
    /// la stessa area (Gelo → rallentamento, Terra → stagger) passano di qui
    /// invece di ricopiare centro/raggio/delay, così un cambio di forma le
    /// segue automaticamente.
    fn emit_area_effect(
        &self,
        params: &AbilityParams,
        ctx: &mut SpellCastContext,
        effect: AoeEffect,
    ) {
        let delay = self.impact_delay();
        ctx.emit_aoe_shaped(
            self.impact_center(params, ctx),
            self.impact_radius(params),
            self.impact_shape(ctx),
            delay,
            delay,
            self.id().as_str().to_string(),
            effect,
        );
    }

    /// Piazza l'impatto ad area del gesto: danno, più lo Stun se il gesto ne
    /// ha uno, più il visual. Entrambe le regioni condividono `impact_delay`,
    /// così il preavviso a terra e ciò che accade quando scade coincidono.
    fn emit_area_impact(&self, params: &AbilityParams, power: f32, ctx: &mut SpellCastContext) {
        self.emit_area_effect(
            params,
            ctx,
            AoeEffect::Damage { amount: power, targeting: AoeTargeting::ExcludeCaster },
        );

        let stun = self.stun_seconds();
        if stun > 0.0 {
            self.emit_area_effect(
                params,
                ctx,
                AoeEffect::CrowdControl {
                    kind: CrowdControlKind::Stun,
                    duration_seconds: stun,
                    once_per_entity: true,
                    targeting: AoeTargeting::ExcludeCaster,
                },
            );
        }

        let center = self.impact_center(params, ctx);
        ctx.emit_visual(self.id().as_str().to_string(), center, center);
    }

    /// Emette danno a `power` nella forma dettata dalla geometria di questa
    /// abilità. Helper condiviso: qualunque Essenza offensiva lo riusa per
    /// non duplicare il dispatch-per-geometria, cambiando solo `power` (es.
    /// Fuoco lo amplifica) — vedi `essences_impl/fuoco.rs`.
    fn emit_damage_for_geometry(&self, power: f32, params: &AbilityParams, ctx: &mut SpellCastContext) {
        match self.geometry() {
            AbilityGeometry::Cone { .. } | AbilityGeometry::Circle { .. } => {
                self.emit_area_impact(params, power, ctx);
            }
            AbilityGeometry::Projectile { range, speed } => {
                // La palla è un'entità replicata che vola davvero: il danno
                // arriva quando arriva lei, non all'istante del lancio.
                match self.projectile_target(ctx) {
                    Some(target) => {
                        ctx.emit_projectile(target, speed, power, PROJECTILE_HIT_RADIUS);
                        let target_position = ctx
                            .potential_targets
                            .iter()
                            .find(|(entity, _)| *entity == target)
                            .map(|(_, position)| *position)
                            .unwrap_or(ctx.caster_position);
                        ctx.emit_visual(self.id().as_str().to_string(), ctx.caster_position, target_position);
                    }
                    None => {
                        // Colpo a vuoto: nessuno davanti. Il gesto si vede
                        // comunque, altrimenti il tasto sembra rotto.
                        let end = ctx.caster_position + flat_direction(ctx.caster_look_direction) * range;
                        ctx.emit_visual(self.id().as_str().to_string(), ctx.caster_position, end);
                    }
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
