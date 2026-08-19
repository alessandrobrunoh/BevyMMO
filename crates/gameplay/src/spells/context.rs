//! Core spell trait and cast context.
//!
//! This module defines the `Spell` trait that all spells must implement,
//! and the `SpellCastContext` that provides contextual information during casting.

use crate::EntityId;
use glam::Vec3;

use super::visuals::SpellVisualEffect;
use crate::effects::{ApplyStatusEffect, EffectBundle, EffectContext, EffectSpec, StatusId};
use crate::stats::components::CombatStats;
use crate::stats::events::ApplyStatModifierEvent;

/// How a spell selects its targets at cast time.
///
/// This enum types what was previously implicit in [`SpellConfig`] constructors,
/// and is used both for input validation (e.g., a `SingleEntity` cast
/// without `target_entity` fails), and for choosing client-side targeting UI
/// (different cursor, circle preview, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetingMode {
    /// Centered on the caster, affects everything in a radius (e.g. `Attack`).
    SelfCentered,
    /// Line-of-sight line shot along look direction (e.g. `RayOfLight`).
    DirectionalLine,
    /// A single selected entity (e.g. `Fireball`).
    SingleEntity,
    /// Ground AoE at the indicated position (e.g. `HealingCircle`).
    GroundAoe,
}

/// Classifies the timing model of a spell for the cast pipeline.
///
/// The value is derived in [`Spell::cast_kind`] from [`SpellConfig`], but can
/// be overridden by implementations when special behavior is needed (rare).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CastKind {
    /// Immediate effect on `just_pressed` (historical behavior).
    #[default]
    Instant,
    /// Blocking wind-up: caster must remain stationary for `cast_time_seconds`
    /// before the effect fires. Movement always cancels the cast.
    CastTime,
    /// Repeated effect as long as the caster holds down the key.
    /// Movement interrupts or not according to [`SpellConfig::channel_movement`].
    Channeling,
}

/// Channeling movement interrupt rule.
///
/// CastTime spells are _always_ interrupted by movement and ignore this
/// enum; it is relevant only for channeling spells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChannelMovementPolicy {
    /// Movement cancels channeling (default for offensive spells).
    #[default]
    InterruptOnMove,
    /// Movement is allowed; only release / re-press / death terminate
    /// channeling (e.g. Swift: must be able to benefit from buff while running).
    AllowMovement,
}

/// Configuration data for a spell.
///
/// Contains static properties that define the spell's behavior and constraints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellConfig {
    /// Cooldown time in seconds after casting.
    pub cooldown_seconds: f32,
    /// Maximum range at which the spell can be cast.
    /// - 0.0 means the spell is centered on the caster
    /// - Higher values allow casting at a distance
    pub cast_range: f32,
    /// Radius of the spell's area of effect.
    /// - Used for spells that affect multiple targets in an area
    /// - 0.0 means single-target or no area component
    pub area_radius: f32,
    /// How the spell selects targets at cast time.
    pub targeting: TargetingMode,
    /// Wind-up duration before effect (0.0 = instant).
    pub cast_time_seconds: f32,
    /// `true` = channeling spell: repeated effect while held.
    pub is_channel: bool,
    /// Movement interrupt policy for channeling. Ignored for
    /// Instant and CastTime spells (which follow fixed Phase 2 rules).
    pub channel_movement: ChannelMovementPolicy,
    /// Optional max duration of channeling. `None` keeps the open-ended
    /// model until client releases key.
    pub channel_duration_seconds: Option<f32>,
}

impl SpellConfig {
    /// Create a new spell configuration.
    pub const fn new(
        cooldown_seconds: f32,
        cast_range: f32,
        area_radius: f32,
        targeting: TargetingMode,
    ) -> Self {
        Self {
            cooldown_seconds,
            cast_range,
            area_radius,
            targeting,
            cast_time_seconds: 0.0,
            is_channel: false,
            channel_movement: ChannelMovementPolicy::InterruptOnMove,
            channel_duration_seconds: None,
        }
    }

    /// Create a configuration for a self-centered melee spell.
    pub const fn melee_aoe(cooldown_seconds: f32, area_radius: f32) -> Self {
        Self::new(
            cooldown_seconds,
            0.0,
            area_radius,
            TargetingMode::SelfCentered,
        )
    }

    /// Create a configuration for a ranged spell hitting along a line
    /// or a single entity: caller must explicitly specify
    /// mode (`DirectionalLine` or `SingleEntity`).
    pub const fn ranged_single_target(
        cooldown_seconds: f32,
        cast_range: f32,
        targeting: TargetingMode,
    ) -> Self {
        Self::new(cooldown_seconds, cast_range, 0.0, targeting)
    }

    /// Create a configuration for a ranged area-of-effect spell placed on the ground.
    pub const fn ranged_aoe(cooldown_seconds: f32, cast_range: f32, area_radius: f32) -> Self {
        Self::new(
            cooldown_seconds,
            cast_range,
            area_radius,
            TargetingMode::GroundAoe,
        )
    }

    /// Builder: sets wind-up duration for a CastTime spell.
    pub const fn with_cast_time(mut self, seconds: f32) -> Self {
        self.cast_time_seconds = seconds;
        self
    }

    /// Builder: turns spell into channeling and sets movement
    /// interrupt policy.
    pub const fn with_channel(mut self, movement_policy: ChannelMovementPolicy) -> Self {
        self.is_channel = true;
        self.channel_movement = movement_policy;
        self
    }

    /// Builder: sets a finite duration for a channeling spell.
    pub const fn with_channel_duration(mut self, seconds: f32) -> Self {
        self.channel_duration_seconds = Some(seconds);
        self
    }
}

/// Target filtering rule applied by [`crate::spells::aoe`] to
/// entities in area. Allows discriminating caster, allies (todo), and enemies
/// without dispatching on `spell_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AoeTargeting {
    /// Hits all entities in range (historical behavior).
    #[default]
    Everyone,
    /// Caster only (e.g. corrected Healing Circle: heals self only).
    CasterOnly,
    /// Everyone except caster (e.g. Meteorite: caster is not damaged).
    ExcludeCaster,
}

impl AoeTargeting {
    /// Returns `true` if `target` is a valid target for this policy,
    /// given the `caster` originating the AoE.
    pub fn allows(self, caster: EntityId, target: EntityId) -> bool {
        match self {
            AoeTargeting::Everyone => true,
            AoeTargeting::CasterOnly => target == caster,
            AoeTargeting::ExcludeCaster => target != caster,
        }
    }
}

/// Homing projectile spawn request, emitted by a spell during cast.
#[derive(Debug, Clone)]
pub struct ProjectileSpawnRequest {
    pub target: EntityId,
    pub speed: f32,
    pub effects: Vec<EffectSpec>,
    pub hit_radius: f32,
}

/// Forma dell'area coperta da una regione AoE.
///
/// Esiste per una ragione precisa: il client disegna l'anteprima di mira con
/// la *stessa* funzione ([`AoeShape::contains`]) che il server usa per
/// decidere chi viene colpito, così il preview non può divergere dall'hitbox.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AoeShape {
    /// Disco pieno attorno al centro.
    #[default]
    Circle,
    /// Settore circolare con l'apice nel centro, aperto di `angle_deg`
    /// COMPLESSIVI (cioè ±`angle_deg / 2`) attorno a `direction`.
    Cone {
        /// Direzione orizzontale normalizzata dell'asse del cono.
        direction: Vec3,
        /// Apertura totale in gradi.
        angle_deg: f32,
    },
}

impl AoeShape {
    /// `true` se `point` cade dentro la forma. Il test è orizzontale: la Y è
    /// scartata come in tutta la matematica di gioco (vedi `flat_offset` in
    /// [`crate::abilities::base_ability`]), perché si combatte su un piano.
    pub fn contains(self, center: Vec3, radius: f32, point: Vec3) -> bool {
        let offset = Vec3::new(point.x - center.x, 0.0, point.z - center.z);
        let distance = offset.length();
        if distance > radius {
            return false;
        }

        match self {
            AoeShape::Circle => true,
            AoeShape::Cone {
                direction,
                angle_deg,
            } => {
                // Un cono di 360° è un cerchio; uno di 0° non colpisce nulla.
                if angle_deg >= 360.0 {
                    return true;
                }
                if angle_deg <= 0.0 {
                    return false;
                }
                let axis = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
                if axis == Vec3::ZERO {
                    // Direzione degenere: senza un asse il cono non ha un
                    // "davanti", quindi non colpisce (meglio del contrario:
                    // un cono che colpisce a 360° per un bug di facing).
                    return false;
                }
                // Chi sta esattamente sull'apice è sempre dentro: l'angolo
                // non è definito e escluderlo sarebbe controintuitivo.
                if distance <= f32::EPSILON {
                    return true;
                }
                (offset / distance).dot(axis) >= (angle_deg.to_radians() / 2.0).cos()
            }
        }
    }
}

/// Spawn request for a persistent Area-of-Effect (AoE) region.
#[derive(Debug, Clone, Default)]
pub struct AoeSpawnRequest {
    pub center: Vec3,
    pub radius: f32,
    /// Forma coperta attorno a `center`. Le spell classiche emettono sempre
    /// [`AoeShape::Circle`]; i gesti Eidolon a cono usano `Cone`.
    pub shape: AoeShape,
    /// Total duration of the region. For "delay + single impact" model
    /// (Meteorite) this equals `initial_delay_seconds` (region despawns
    /// immediately after applying effect).
    pub duration_seconds: f32,
    /// Delay time before effect is first applied.
    /// During this interval region exists (useful for Meteorite's visual
    /// warning marker) but applies no damage/heal/modifier. Default `0.0`.
    pub initial_delay_seconds: f32,
    pub spell_id: String,
    /// Generic effects for damage/heal/status payloads.
    pub effects: Vec<EffectSpec>,
    /// Targeting policy. Defaults to [`AoeTargeting::Everyone`] for backward
    /// compatibility with existing call sites that do not specify it.
    pub targeting: AoeTargeting,
}

/// Context provided to a spell during casting.
///
/// This context contains all the information a spell needs to make decisions
/// about what targets to affect and what effects to apply. Spells collect
/// their pending damage/healing events in the context, which are then drained
/// and applied by the casting system.
pub struct SpellCastContext<'a> {
    /// The entity casting the spell.
    pub caster: EntityId,
    /// The current position of the caster.
    pub caster_position: Vec3,
    /// The combat stats of the caster (for damage calculations).
    pub caster_combat: &'a CombatStats,
    /// Horizontal direction the caster is facing, resolved server-side.
    pub caster_look_direction: Vec3,
    /// The optional target position for the spell.
    pub target_position: Option<Vec3>,
    /// The optional target entity for homing/projectile spells.
    pub target_entity: Option<EntityId>,
    /// List of potential targets in range, with their positions.
    ///
    /// This is provided as a slice of (EntityId, Vec3) tuples. Spells can filter
    /// this list based on their own criteria (range, area of effect, etc.).
    pub potential_targets: &'a [(EntityId, Vec3)],
    /// Unified effects emitted by the spell. These are resolved authoritatively
    /// after validation; the older typed lists remain as compatibility adapters.
    pub pending_effects: Vec<EffectBundle>,

    /// Pending projectile spawn requests.
    pub pending_projectiles: Vec<ProjectileSpawnRequest>,
    /// Pending AoE spawn requests.
    pub pending_aoes: Vec<AoeSpawnRequest>,
    /// Pending single-target stat modifier requests (buff/debuff applied
    /// outside an AoE, e.g. Swift self-buff).
    pub pending_modifiers: Vec<ApplyStatModifierEvent>,
    /// Pending replicated visual effects to broadcast to clients after validation.
    pub pending_visuals: Vec<SpellVisualEffect>,
}

impl<'a> SpellCastContext<'a> {
    /// Create a new spell cast context.
    pub fn new(
        caster: EntityId,
        caster_position: Vec3,
        caster_combat: &'a CombatStats,
        caster_look_direction: Vec3,
        target_position: Option<Vec3>,
        target_entity: Option<EntityId>,
        potential_targets: &'a [(EntityId, Vec3)],
    ) -> Self {
        Self {
            caster,
            caster_position,
            caster_combat,
            caster_look_direction,
            target_position,
            target_entity,
            potential_targets,
            pending_effects: Vec::new(),

            pending_projectiles: Vec::new(),
            pending_aoes: Vec::new(),
            pending_modifiers: Vec::new(),
            pending_visuals: Vec::new(),
        }
    }

    pub fn emit_cleanse(&mut self, target: EntityId, effect: crate::effects::CleanseEffect) {
        self.emit_effect(target, EffectSpec::Cleanse(effect));
    }

    pub fn emit_purge(&mut self, target: EntityId, effect: crate::effects::PurgeEffect) {
        self.emit_effect(target, EffectSpec::Purge(effect));
    }

    pub fn emit_effect(&mut self, target: EntityId, effect: EffectSpec) {
        let mut context = EffectContext::new(target);
        context.source = Some(self.caster);
        self.pending_effects
            .push(EffectBundle::single(context, effect));
    }

    /// Emit a unified status application request.
    pub fn emit_status(&mut self, target: EntityId, status_id: StatusId) {
        let mut context = EffectContext::new(target);
        context.source = Some(self.caster);
        self.pending_effects.push(EffectBundle::single(
            context,
            EffectSpec::ApplyStatus(ApplyStatusEffect {
                status_id,
                duration_override_seconds: None,
                potency: 1.0,
            }),
        ));
    }

    /// Emit a projectile spawn request (for homing/projectile spells).
    pub fn emit_projectile(
        &mut self,
        target: EntityId,
        speed: f32,
        effects: Vec<EffectSpec>,
        hit_radius: f32,
    ) {
        self.pending_projectiles.push(ProjectileSpawnRequest {
            target,
            speed,
            effects,
            hit_radius,
        });
    }

    /// Compatibility alias for content that uses the explicit effects name.
    pub fn emit_projectile_effects(
        &mut self,
        target: EntityId,
        speed: f32,
        effects: Vec<EffectSpec>,
        hit_radius: f32,
    ) {
        self.emit_projectile(target, speed, effects, hit_radius);
    }

    /// Emit an AoE spawn request with full control over shape and timing.
    #[allow(clippy::too_many_arguments)]
    pub fn emit_aoe(
        &mut self,
        center: Vec3,
        radius: f32,
        shape: AoeShape,
        duration_seconds: f32,
        initial_delay_seconds: f32,
        spell_id: impl Into<String>,
        effects: Vec<EffectSpec>,
    ) {
        self.emit_aoe_with_targeting(
            center,
            radius,
            shape,
            duration_seconds,
            initial_delay_seconds,
            spell_id,
            effects,
            AoeTargeting::default(),
        );
    }

    /// Emit an offensive AoE that never applies its effects to the caster.
    #[allow(clippy::too_many_arguments)]
    pub fn emit_aoe_excluding_caster(
        &mut self,
        center: Vec3,
        radius: f32,
        shape: AoeShape,
        duration_seconds: f32,
        initial_delay_seconds: f32,
        spell_id: impl Into<String>,
        effects: Vec<EffectSpec>,
    ) {
        self.emit_aoe_with_targeting(
            center,
            radius,
            shape,
            duration_seconds,
            initial_delay_seconds,
            spell_id,
            effects,
            AoeTargeting::ExcludeCaster,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_aoe_with_targeting(
        &mut self,
        center: Vec3,
        radius: f32,
        shape: AoeShape,
        duration_seconds: f32,
        initial_delay_seconds: f32,
        spell_id: impl Into<String>,
        effects: Vec<EffectSpec>,
        targeting: AoeTargeting,
    ) {
        self.pending_aoes.push(AoeSpawnRequest {
            center,
            radius,
            shape,
            duration_seconds,
            initial_delay_seconds,
            spell_id: spell_id.into(),
            effects,
            targeting,
        });
    }

    /// Convenience wrapper for circular AoE (no delay).
    pub fn emit_aoe_circle(
        &mut self,
        center: Vec3,
        radius: f32,
        duration_seconds: f32,
        spell_id: impl Into<String>,
        effects: Vec<EffectSpec>,
    ) {
        self.emit_aoe(
            center,
            radius,
            AoeShape::Circle,
            duration_seconds,
            0.0,
            spell_id,
            effects,
        );
    }

    /// Emit a replicated visual effect after a successful server-side cast.
    pub fn emit_visual(&mut self, spell_id: impl Into<String>, start: Vec3, end: Vec3) {
        self.pending_visuals.push(SpellVisualEffect {
            spell_id: spell_id.into(),
            start,
            end,
        });
    }

    /// Filter potential targets by distance from a center point.
    pub fn targets_in_radius(&self, center: Vec3, radius: f32) -> Vec<(EntityId, Vec3)> {
        self.potential_targets
            .iter()
            .filter(|(_, pos)| {
                let distance = center.distance(*pos);
                distance <= radius
            })
            .map(|(entity, pos)| (*entity, *pos))
            .collect()
    }

    /// Get the effective center for the spell's area of effect.
    ///
    /// Returns `target_position` if set, otherwise `caster_position`.
    pub fn effective_center(&self) -> Vec3 {
        self.target_position.unwrap_or(self.caster_position)
    }
}

/// Trait that all spells must implement.
///
/// This trait defines the interface for spell behavior. Implementations are
/// responsible for:
/// - Providing identification and display information
/// - Providing static configuration (cooldown, range, area)
/// - Implementing the actual spell logic in the `cast` method
pub trait Spell: Send + Sync + 'static {
    /// Get the unique identifier for this spell.
    fn id(&self) -> crate::spells::registry::SpellId;

    /// Get the human-readable display name for this spell.
    fn display_name(&self) -> &'static str;

    /// Get the static configuration for this spell.
    fn config(&self) -> SpellConfig;

    /// Classifies the timing model of this spell for the cast pipeline
    /// (Phase 2). Default implementation derives the value from
    /// [`SpellConfig`] and covers the vast majority of cases.
    fn cast_kind(&self) -> CastKind {
        let config = self.config();
        if config.is_channel {
            CastKind::Channeling
        } else if config.cast_time_seconds > 0.0 {
            CastKind::CastTime
        } else {
            CastKind::Instant
        }
    }

    /// Accumulation interval between channeling ticks.
    /// Relevant only for channeling spells: central system accumulates elapsed
    /// time and invokes [`cast`](Self::cast) only when exceeding this
    /// interval. Default `0.25s`.
    fn channel_tick_interval_seconds(&self) -> f32 {
        0.25
    }

    /// Execute the spell's logic.
    ///
    /// This method is called when the spell is successfully cast. It receives
    /// a mutable context that contains:
    /// - Caster information (entity, position, combat stats)
    /// - Target information (if applicable)
    /// - Potential targets that can be affected
    ///
    /// The spell should:
    /// 1. Filter/select targets based on its criteria
    /// - Emit unified effects via the context
    /// 3. Return; the system will apply the events and handle cooldowns
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn cast(&self, ctx: &mut SpellCastContext) {
    ///     let center = ctx.effective_center();
    ///     let targets = ctx.targets_in_radius(center, self.config().area_radius);
    ///
    ///     for (target, _) in targets {
    ///         if target != ctx.caster {
    ///             ctx.emit_effect(target, EffectSpec::Damage(
    ///                 crate::effects::DamageEffect { amount: ctx.caster_combat.attack_power },
    ///             ));
    ///         }
    ///     }
    /// }
    /// ```
    fn cast(&self, ctx: &mut SpellCastContext);
}

/// Delegation trait for the cast logic of spells declared with `#[spell(...)]`.
///
/// The `#[spell(...)]` macro generates `impl Spell` for all static metadata
/// (`id`, `display_name`, `config`) and delegates `Spell::cast` to this trait.
/// Implement `SpellCast` on your struct to provide the actual cast behavior,
/// exactly as `EssenceEffect` / `ModifierEffect` / `AncientWordEffect` work
/// for the corresponding Glifo macros.
pub trait SpellCast {
    fn cast(&self, ctx: &mut SpellCastContext);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cono di 70° puntato lungo +Z, apice nell'origine, raggio 6.
    fn wave() -> (Vec3, f32, AoeShape) {
        (
            Vec3::ZERO,
            6.0,
            AoeShape::Cone {
                direction: Vec3::Z,
                angle_deg: 70.0,
            },
        )
    }

    #[test]
    fn circle_only_checks_distance() {
        let shape = AoeShape::Circle;
        assert!(shape.contains(Vec3::ZERO, 3.0, Vec3::new(0.0, 0.0, 2.9)));
        // Dietro, ma dentro il raggio: un cerchio colpisce comunque.
        assert!(shape.contains(Vec3::ZERO, 3.0, Vec3::new(0.0, 0.0, -2.9)));
        assert!(!shape.contains(Vec3::ZERO, 3.0, Vec3::new(0.0, 0.0, 3.1)));
    }

    #[test]
    fn circle_ignores_height() {
        // La Y è scartata: un bersaglio "sopra" resta dentro l'area.
        assert!(AoeShape::Circle.contains(Vec3::ZERO, 3.0, Vec3::new(1.0, 50.0, 1.0)));
    }

    #[test]
    fn cone_hits_straight_ahead() {
        let (center, radius, shape) = wave();
        assert!(shape.contains(center, radius, Vec3::new(0.0, 0.0, 5.0)));
    }

    #[test]
    fn cone_misses_behind_the_caster() {
        let (center, radius, shape) = wave();
        assert!(!shape.contains(center, radius, Vec3::new(0.0, 0.0, -5.0)));
    }

    #[test]
    fn cone_misses_at_ninety_degrees() {
        // 90° dall'asse è fuori da un cono di 70° (che copre ±35°).
        let (center, radius, shape) = wave();
        assert!(!shape.contains(center, radius, Vec3::new(5.0, 0.0, 0.0)));
    }

    #[test]
    fn cone_edge_is_inside_and_just_past_it_is_not() {
        let (center, radius, shape) = wave();
        let inside = 34.0_f32.to_radians();
        let outside = 36.0_f32.to_radians();
        assert!(shape.contains(
            center,
            radius,
            Vec3::new(inside.sin() * 4.0, 0.0, inside.cos() * 4.0)
        ));
        assert!(!shape.contains(
            center,
            radius,
            Vec3::new(outside.sin() * 4.0, 0.0, outside.cos() * 4.0)
        ));
    }

    #[test]
    fn cone_respects_the_radius() {
        let (center, radius, shape) = wave();
        assert!(!shape.contains(center, radius, Vec3::new(0.0, 0.0, 6.5)));
    }

    #[test]
    fn cone_with_a_degenerate_axis_hits_nothing() {
        // Facing non risolto: meglio non colpire nulla che colpire a 360°.
        let shape = AoeShape::Cone {
            direction: Vec3::ZERO,
            angle_deg: 70.0,
        };
        assert!(!shape.contains(Vec3::ZERO, 6.0, Vec3::new(0.0, 0.0, 1.0)));
    }

    #[test]
    fn cone_includes_whoever_stands_on_the_apex() {
        let (center, radius, shape) = wave();
        assert!(shape.contains(center, radius, center));
    }

    #[test]
    fn emit_status_queues_a_unified_apply_status_effect() {
        use crate::effects::EffectSpec;
        use crate::stats::components::CombatStats;

        let combat = CombatStats {
            attack_power: 0.0,
            armor: 0.0,
        };
        let caster = EntityId::new(1);
        let target = EntityId::new(2);
        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &[]);

        ctx.emit_status(target, StatusId::new("stun"));

        assert_eq!(ctx.pending_effects.len(), 1);
        assert!(matches!(
            ctx.pending_effects[0].effects[0],
            EffectSpec::ApplyStatus(_)
        ));
        assert_eq!(ctx.pending_effects[0].context.source, Some(caster));
        assert_eq!(ctx.pending_effects[0].context.target, target);
    }

    #[test]
    fn emit_cleanse_and_purge_queue_unified_effects() {
        use crate::effects::{
            CleanseEffect, EffectSpec, PurgeEffect, StatusFilter, StatusSelection,
        };
        use crate::stats::components::CombatStats;

        let combat = CombatStats {
            attack_power: 0.0,
            armor: 0.0,
        };
        let caster = EntityId::new(1);
        let target = EntityId::new(2);
        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &[]);

        ctx.emit_cleanse(
            target,
            CleanseEffect {
                filter: StatusFilter::Debuffs,
                max_statuses: Some(2),
                selection: StatusSelection::Oldest,
            },
        );
        ctx.emit_purge(
            target,
            PurgeEffect {
                filter: StatusFilter::Buffs,
                max_statuses: Some(1),
                selection: StatusSelection::Newest,
            },
        );

        assert!(matches!(
            ctx.pending_effects[0].effects[0],
            EffectSpec::Cleanse(CleanseEffect {
                filter: StatusFilter::Debuffs,
                max_statuses: Some(2),
                selection: StatusSelection::Oldest,
            })
        ));
        assert!(matches!(
            ctx.pending_effects[1].effects[0],
            EffectSpec::Purge(PurgeEffect {
                filter: StatusFilter::Buffs,
                max_statuses: Some(1),
                selection: StatusSelection::Newest,
            })
        ));
        assert!(ctx
            .pending_effects
            .iter()
            .all(|bundle| bundle.context.source == Some(caster)));
    }

    #[test]
    fn emit_aoe_defaults_to_a_circle() {
        use crate::effects::EffectSpec;
        use crate::stats::components::CombatStats;

        let combat = CombatStats {
            attack_power: 0.0,
            armor: 0.0,
        };
        let caster = EntityId::new(1);
        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &[]);

        ctx.emit_aoe_circle(
            Vec3::ZERO,
            3.0,
            1.0,
            "test",
            vec![EffectSpec::Damage(crate::effects::DamageEffect {
                amount: 1.0,
            })],
        );

        assert_eq!(ctx.pending_aoes[0].shape, AoeShape::Circle);
    }

    #[test]
    fn offensive_aoe_excludes_the_caster() {
        use crate::effects::EffectSpec;
        use crate::stats::components::CombatStats;

        let combat = CombatStats {
            attack_power: 0.0,
            armor: 0.0,
        };
        let caster = EntityId::new(1);
        let mut ctx = SpellCastContext::new(caster, Vec3::ZERO, &combat, Vec3::Z, None, None, &[]);

        ctx.emit_aoe_excluding_caster(
            Vec3::ZERO,
            3.0,
            AoeShape::Circle,
            0.0,
            0.0,
            "offensive_test",
            vec![EffectSpec::Damage(crate::effects::DamageEffect {
                amount: 1.0,
            })],
        );

        assert_eq!(ctx.pending_aoes[0].targeting, AoeTargeting::ExcludeCaster);
        assert!(!ctx.pending_aoes[0].targeting.allows(caster, caster));
    }
}
