//! Core spell trait and cast context.
//!
//! This module defines the `Spell` trait that all spells must implement,
//! and the `SpellCastContext` that provides contextual information during casting.

use bevy::ecs::entity::Entity;
use bevy::math::Vec3;

use crate::stats::components::CombatStats;
use crate::stats::events::{DamageEvent, HealEvent};

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
}

impl SpellConfig {
    /// Create a new spell configuration.
    pub const fn new(cooldown_seconds: f32, cast_range: f32, area_radius: f32) -> Self {
        Self {
            cooldown_seconds,
            cast_range,
            area_radius,
        }
    }

    /// Create a configuration for a self-centered melee spell.
    pub const fn melee_aoe(cooldown_seconds: f32, area_radius: f32) -> Self {
        Self {
            cooldown_seconds,
            cast_range: 0.0,
            area_radius,
        }
    }

    /// Create a configuration for a ranged single-target spell.
    pub const fn ranged_single_target(cooldown_seconds: f32, cast_range: f32) -> Self {
        Self {
            cooldown_seconds,
            cast_range,
            area_radius: 0.0,
        }
    }

    /// Create a configuration for a ranged area-of-effect spell.
    pub const fn ranged_aoe(cooldown_seconds: f32, cast_range: f32, area_radius: f32) -> Self {
        Self {
            cooldown_seconds,
            cast_range,
            area_radius,
        }
    }
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
    /// The optional target position for the spell.
    pub target_position: Option<Vec3>,
    /// List of potential targets in range, with their positions.
    ///
    /// This is provided as a slice of (Entity, Vec3) tuples. Spells can filter
    /// this list based on their own criteria (range, area of effect, etc.).
    pub potential_targets: &'a [(Entity, Vec3)],
    /// Pending damage events to be applied after the spell cast completes.
    pub pending_damage: Vec<DamageEvent>,
    /// Pending healing events to be applied after the spell cast completes.
    pub pending_healing: Vec<HealEvent>,
}

impl<'a> SpellCastContext<'a> {
    /// Create a new spell cast context.
    pub fn new(
        caster: Entity,
        caster_position: Vec3,
        caster_combat: &'a CombatStats,
        target_position: Option<Vec3>,
        potential_targets: &'a [(Entity, Vec3)],
    ) -> Self {
        Self {
            caster,
            caster_position,
            caster_combat,
            target_position,
            potential_targets,
            pending_damage: Vec::new(),
            pending_healing: Vec::new(),
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
    fn id(&self) -> crate::plugins::spells::registry::SpellId;

    /// Get the human-readable display name for this spell.
    fn display_name(&self) -> &'static str;

    /// Get the static configuration for this spell.
    fn config(&self) -> SpellConfig;

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
    /// ```rust
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
