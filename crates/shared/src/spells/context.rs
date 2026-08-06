//! Core spell trait and cast context.
//!
//! This module defines the `Spell` trait that all spells must implement,
//! and the `SpellCastContext` that provides contextual information during casting.

use bevy::ecs::entity::Entity;
use bevy::math::Vec3;

use crate::network::protocol::SpellVisualEffect;
use crate::stats::components::CombatStats;
use crate::stats::events::{
    ApplyStatModifierEvent, DamageEvent, HealEvent, ModifierEffect, ModifierKind,
};

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
    pub fn allows(self, caster: Entity, target: Entity) -> bool {
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
    pub target: Entity,
    pub speed: f32,
    pub damage: f32,
    pub hit_radius: f32,
}

/// Effect applied by an AoE region to entities entering it.
///
/// Lives in spawn request payload (as already done for
/// projectiles with `ProjectileSpawnRequest`), so the central system
/// `update_aoe_regions` remains generic and ignores spell identity.
#[derive(Debug, Clone, PartialEq)]
pub enum AoeEffect {
    /// Applies an [`ApplyStatModifierEvent`] to entities in area.
    ApplyModifier {
        effects: Vec<ModifierEffect>,
        /// `None` = permanent until explicitly removed.
        duration_seconds: Option<f32>,
        kind: ModifierKind,
        /// `true`: each entity receives effect only once (e.g. healing
        /// circle: enters → buff applied, then ignored).
        /// `false`: effect is re-applied as long as entity stays inside
        /// (e.g. poison cloud doing continuous DoT).
        once_per_entity: bool,
        /// Filters which entities are valid targets relative to caster.
        targeting: AoeTargeting,
    },
    /// Damage burst applied one-off to entities in area at moment
    /// of impact (or entry, if not delayed). Used by "bomb-style"
    /// spells like Meteorite.
    Damage {
        amount: f32,
        targeting: AoeTargeting,
    },
    /// Heal burst applied one-off to entities in area.
    Heal {
        amount: f32,
        targeting: AoeTargeting,
    },
    /// Applies a Crowd Control effect (e.g. Stun) to entities in area
    /// at moment of impact. Effect duration then lives on target,
    /// independent of AoE region lifetime.
    CrowdControl {
        kind: crate::crowd_control::CrowdControlKind,
        duration_seconds: f32,
        /// `true`: each entity receives effect only once (typical for
        /// one-off AoE bursts like Stun Field).
        once_per_entity: bool,
        targeting: AoeTargeting,
    },
}

impl AoeEffect {
    /// Returns targeting policy associated with effect.
    pub fn targeting(&self) -> AoeTargeting {
        match self {
            AoeEffect::ApplyModifier { targeting, .. }
            | AoeEffect::Damage { targeting, .. }
            | AoeEffect::Heal { targeting, .. }
            | AoeEffect::CrowdControl { targeting, .. } => *targeting,
        }
    }
}

/// Spawn request for a persistent Area-of-Effect (AoE) region.
#[derive(Debug, Clone)]
pub struct AoeSpawnRequest {
    pub center: Vec3,
    pub radius: f32,
    /// Total duration of the region. For "delay + single impact" model
    /// (Meteorite) this equals `initial_delay_seconds` (region despawns
    /// immediately after applying effect).
    pub duration_seconds: f32,
    /// Delay time before effect is first applied.
    /// During this interval region exists (useful for Meteorite's visual
    /// warning marker) but applies no damage/heal/modifier. Default `0.0`.
    pub initial_delay_seconds: f32,
    pub spell_id: String,
    pub effect: AoeEffect,
}

/// Context provided to a spell during casting.
///
/// This context contains all the information a spell needs to make decisions
/// about what targets to affect and what effects to apply. Spells collect
/// their pending damage/healing events in the context, which are then drained
/// and applied by the casting system.
pub struct SpellCastContext<'a> {
    /// The entity casting the spell.
    pub caster: Entity,
    /// The current position of the caster.
    pub caster_position: Vec3,
    /// The combat stats of the caster (for damage calculations).
    pub caster_combat: &'a CombatStats,
    /// Horizontal direction the caster is facing, resolved server-side.
    pub caster_look_direction: Vec3,
    /// The optional target position for the spell.
    pub target_position: Option<Vec3>,
    /// The optional target entity for homing/projectile spells.
    pub target_entity: Option<Entity>,
    /// List of potential targets in range, with their positions.
    ///
    /// This is provided as a slice of (Entity, Vec3) tuples. Spells can filter
    /// this list based on their own criteria (range, area of effect, etc.).
    pub potential_targets: &'a [(Entity, Vec3)],
    /// Pending damage events to be applied after the spell cast completes.
    pub pending_damage: Vec<DamageEvent>,
    /// Pending healing events to be applied after the spell cast completes.
    pub pending_healing: Vec<HealEvent>,
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
        caster: Entity,
        caster_position: Vec3,
        caster_combat: &'a CombatStats,
        caster_look_direction: Vec3,
        target_position: Option<Vec3>,
        target_entity: Option<Entity>,
        potential_targets: &'a [(Entity, Vec3)],
    ) -> Self {
        Self {
            caster,
            caster_position,
            caster_combat,
            caster_look_direction,
            target_position,
            target_entity,
            potential_targets,
            pending_damage: Vec::new(),
            pending_healing: Vec::new(),
            pending_projectiles: Vec::new(),
            pending_aoes: Vec::new(),
            pending_modifiers: Vec::new(),
            pending_visuals: Vec::new(),
        }
    }

    /// Emit a damage event to a target.
    pub fn emit_damage(&mut self, target: Entity, amount: f32) {
        self.pending_damage.push(DamageEvent {
            target,
            source: Some(self.caster),
            amount,
        });
    }

    /// Emit a healing event to a target.
    pub fn emit_heal(&mut self, target: Entity, amount: f32) {
        self.pending_healing.push(HealEvent {
            target,
            source: Some(self.caster),
            amount,
        });
    }

    /// Emit a projectile spawn request (for homing/projectile spells).
    pub fn emit_projectile(&mut self, target: Entity, speed: f32, damage: f32, hit_radius: f32) {
        self.pending_projectiles.push(ProjectileSpawnRequest {
            target,
            speed,
            damage,
            hit_radius,
        });
    }

    /// Emit an AoE spawn request.
    ///
    /// `effect` carries the effect payload (damage, heal, modifier,
    /// etc.) so the central system does not need to dispatch on
    /// `spell_id`.
    pub fn emit_aoe(
        &mut self,
        center: Vec3,
        radius: f32,
        duration_seconds: f32,
        spell_id: impl Into<String>,
        effect: AoeEffect,
    ) {
        self.emit_aoe_with_delay(center, radius, duration_seconds, 0.0, spell_id, effect);
    }

    /// Like [`emit_aoe`] but with an initial delay before effect fires
    /// (e.g. Meteorite: 2s red warning circle before impact).
    pub fn emit_aoe_with_delay(
        &mut self,
        center: Vec3,
        radius: f32,
        duration_seconds: f32,
        initial_delay_seconds: f32,
        spell_id: impl Into<String>,
        effect: AoeEffect,
    ) {
        self.pending_aoes.push(AoeSpawnRequest {
            center,
            radius,
            duration_seconds,
            initial_delay_seconds,
            spell_id: spell_id.into(),
            effect,
        });
    }

    /// Emit a one-shot stat modifier (buff/debuff) on a single target.
    ///
    /// Utility wrapper around [`ApplyStatModifierEvent`]: spells that only
    /// need to apply a buff to caster (e.g. Swift) do not need to manually
    /// construct the event.
    pub fn emit_modifier(
        &mut self,
        target: Entity,
        effects: Vec<ModifierEffect>,
        duration_seconds: Option<f32>,
        kind: ModifierKind,
    ) {
        self.pending_modifiers.push(ApplyStatModifierEvent {
            target,
            source: Some(self.caster),
            effects,
            duration_seconds,
            kind,
        });
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
    pub fn targets_in_radius(&self, center: Vec3, radius: f32) -> Vec<(Entity, Vec3)> {
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
    /// 2. Emit damage/healing events via the context
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
    ///             ctx.emit_damage(target, ctx.caster_combat.attack_power);
    ///         }
    ///     }
    /// }
    /// ```
    fn cast(&self, ctx: &mut SpellCastContext);
}
