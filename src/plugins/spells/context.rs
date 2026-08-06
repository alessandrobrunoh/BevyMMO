//! Core spell trait and cast context.
//!
//! This module defines the `Spell` trait that all spells must implement,
//! and the `SpellCastContext` that provides contextual information during casting.

use bevy::ecs::entity::Entity;
use bevy::math::Vec3;

use crate::network::protocol::SpellVisualEffect;
use crate::stats::components::CombatStats;
use crate::stats::events::{DamageEvent, HealEvent, ModifierEffect, ModifierKind};

/// Come una spell seleziona i propri bersagli al momento del cast.
///
/// Questo enum tipizza ciò che prima era implicito nei costruttori di
/// [`SpellConfig`], e viene usato sia per validare l'input (es. un cast di
/// `SingleEntity` senza `target_entity` fallisce), sia per scegliere la UI di
/// targeting lato client (cursor diverso, preview del cerchio, ecc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetingMode {
    /// Centrata sul caster, colpisce tutto in un raggio (es. `Attack`).
    SelfCentered,
    /// Tiro in linea d'aria lungo la direzione di sguardo (es. `Fireball`).
    DirectionalLine,
    /// Una singola entità selezionata (es. `Followball`).
    SingleEntity,
    /// AoE a terra nella posizione indicata (es. `HealingCircle`).
    GroundAoe,
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
    /// Modalità con cui la spell seleziona i bersagli al momento del cast.
    pub targeting: TargetingMode,
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

    /// Create a configuration for a ranged spell che colpisce lungo una linea
    /// o una singola entità: il chiamante deve specificare esplicitamente la
    /// modalità (`DirectionalLine` o `SingleEntity`).
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
}

/// Richiesta di spawn di un proiettile homing, emessa da una spell durante il cast.
#[derive(Debug, Clone)]
pub struct ProjectileSpawnRequest {
    pub target: Entity,
    pub speed: f32,
    pub damage: f32,
    pub hit_radius: f32,
}

/// Effetto applicato da una regione AoE alle entità che vi entrano.
///
/// Vive nel payload della richiesta di spawn (come già avviene per i
/// proiettili con `ProjectileSpawnRequest`), in modo che il sistema centrale
/// `update_aoe_regions` resti generico e ignori l'identità della spell.
#[derive(Debug, Clone, PartialEq)]
pub enum AoeEffect {
    /// Applica un [`ApplyStatModifierEvent`] alle entità nell'area.
    ApplyModifier {
        effects: Vec<ModifierEffect>,
        /// `None` = permanente finché non rimosso esplicitamente.
        duration_seconds: Option<f32>,
        kind: ModifierKind,
        /// `true`: ogni entità riceve l'effetto una sola volta (es. healing
        /// circle: entra → buff applicato, poi ignorata).
        /// `false`: l'effetto viene ri-applicato finché l'entità resta dentro
        /// (es. poison cloud che fa DoT continuo).
        once_per_entity: bool,
    },
}

/// Richiesta di spawn di una regione ad area (AoE) persistente.
#[derive(Debug, Clone)]
pub struct AoeSpawnRequest {
    pub center: Vec3,
    pub radius: f32,
    pub duration_seconds: f32,
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
    /// `effect` porta con sé il payload dell'effetto (danno, cura, modifier,
    /// ecc.) in modo che il sistema centrale non debba fare dispatch su
    /// `spell_id`.
    pub fn emit_aoe(
        &mut self,
        center: Vec3,
        radius: f32,
        duration_seconds: f32,
        spell_id: impl Into<String>,
        effect: AoeEffect,
    ) {
        self.pending_aoes.push(AoeSpawnRequest {
            center,
            radius,
            duration_seconds,
            spell_id: spell_id.into(),
            effect,
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
