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

/// Classifica il modello temporale di una spell ai fini del pipeline di cast.
///
/// Il valore è derivato in [`Spell::cast_kind`] dalla [`SpellConfig`], ma può
/// essere sovrascritto dalle implementazioni quando serve un comportamento
/// speciale (raro).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CastKind {
    /// Effetto immediato allo `just_pressed` (comportamento storico).
    #[default]
    Instant,
    /// Wind-up bloccante: il caster deve restare fermo per `cast_time_seconds`
    /// prima che l'effetto parta. Il movimento interrompe sempre il cast.
    CastTime,
    /// Effetto ripetuto finché il caster mantiene premuto il tasto. Il
    /// movimento interrompe o meno in base a [`SpellConfig::channel_movement`].
    Channeling,
}

/// Regola di interruzione del channeling in funzione del movimento.
///
/// Le spell CastTime sono _sempre_ interrotte dal movimento e ignorano questo
/// enum; è rilevante solo per le spell channeling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChannelMovementPolicy {
    /// Il movimento cancella il channeling (default per spell offensive).
    #[default]
    InterruptOnMove,
    /// Il movimento è consentito; solo release / re-press / morte terminano
    /// il channeling (es. Swift: devi poter beneficiare del buff mentre corri).
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
    /// Modalità con cui la spell seleziona i bersagli al momento del cast.
    pub targeting: TargetingMode,
    /// Durata del wind-up prima dell'effetto (0.0 = istantanea).
    pub cast_time_seconds: f32,
    /// `true` = spell channeling: effetto ripetuto finché rilasciato.
    pub is_channel: bool,
    /// Policy di interruzione del channeling col movimento. Ignorata per le
    /// spell Instant e CastTime (per le quali vale la regola fissa di Phase 2).
    pub channel_movement: ChannelMovementPolicy,
    /// Durata massima opzionale di un channeling. `None` mantiene il modello
    /// open-ended finché il client rilascia il tasto.
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

    /// Builder: imposta il wind-up di una spell CastTime.
    pub const fn with_cast_time(mut self, seconds: f32) -> Self {
        self.cast_time_seconds = seconds;
        self
    }

    /// Builder: trasforma la spell in channeling e imposta la policy di
    /// interruzione col movimento.
    pub const fn with_channel(mut self, movement_policy: ChannelMovementPolicy) -> Self {
        self.is_channel = true;
        self.channel_movement = movement_policy;
        self
    }

    /// Builder: imposta una durata finita per una spell channeling.
    pub const fn with_channel_duration(mut self, seconds: f32) -> Self {
        self.channel_duration_seconds = Some(seconds);
        self
    }
}

/// Regola di filtraggio target applicata da [`crate::plugins::spells::aoe`] alle
/// entità nell'area. Permette di discriminare caster, alleati (todo) e nemici
/// senza dover dispatchare sul `spell_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AoeTargeting {
    /// Colpise tutte le entità in range (comportamento storico).
    #[default]
    Everyone,
    /// Solo il caster (es. Healing Circle corretto: cura solo se stesso).
    CasterOnly,
    /// Tutti tranne il caster (es. Meteorite: il caster non si danneggia).
    ExcludeCaster,
}

impl AoeTargeting {
    /// Restituisce `true` se `target` è un bersaglio valido per questa policy,
    /// dato il `caster` che ha originato l'AoE.
    pub fn allows(self, caster: Entity, target: Entity) -> bool {
        match self {
            AoeTargeting::Everyone => true,
            AoeTargeting::CasterOnly => target == caster,
            AoeTargeting::ExcludeCaster => target != caster,
        }
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
        /// Filtra quali entità sono bersagli valide rispetto al caster.
        targeting: AoeTargeting,
    },
    /// Burst di danno applicato una tantum alle entità nell'area al momento
    /// dell'impatto (o all'ingresso, se non delaytato). Usato da spell
    /// "bomb-style" come Meteorite.
    Damage {
        amount: f32,
        targeting: AoeTargeting,
    },
    /// Burst di cura applicato una tantum alle entità nell'area.
    Heal {
        amount: f32,
        targeting: AoeTargeting,
    },
}

impl AoeEffect {
    /// Restituisce la policy di targeting associata all'effetto.
    pub fn targeting(&self) -> AoeTargeting {
        match self {
            AoeEffect::ApplyModifier { targeting, .. }
            | AoeEffect::Damage { targeting, .. }
            | AoeEffect::Heal { targeting, .. } => *targeting,
        }
    }
}

/// Richiesta di spawn di una regione ad area (AoE) persistente.
#[derive(Debug, Clone)]
pub struct AoeSpawnRequest {
    pub center: Vec3,
    pub radius: f32,
    /// Durata totale della regione. Per il modello "delay + impatto singolo"
    /// (Meteorite) equivale a `initial_delay_seconds` (la regione despawna
    /// subito dopo aver applicato l'effetto).
    pub duration_seconds: f32,
    /// Tempo di delay prima che l'effetto venga applicato la prima volta.
    /// Durante questo intervallo la regione esiste (utile per il visual marker
    /// del Meteorite) ma non applica damage/heal/modifier. Default `0.0`.
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
    /// Pending single-target stat modifier requests (buff/debuff applicati
    /// fuori da una AoE, es. self-buff di Swift).
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
        self.emit_aoe_with_delay(center, radius, duration_seconds, 0.0, spell_id, effect);
    }

    /// Come [`emit_aoe`] ma con un delay iniziale prima che l'effetto parta
    /// (es. Meteorite: 2s di cerchio rosso di warning prima dell'impatto).
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
    /// Utility wrapper around [`ApplyStatModifierEvent`]: le spell che devono
    /// solo applicare un buff al caster (es. Swift) non devono costruire
    /// manualmente l'evento.
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
    fn id(&self) -> crate::plugins::spells::registry::SpellId;

    /// Get the human-readable display name for this spell.
    fn display_name(&self) -> &'static str;

    /// Get the static configuration for this spell.
    fn config(&self) -> SpellConfig;

    /// Classifica il modello temporale di questa spell ai fini del pipeline
    /// di cast (Phase 2). L'implementazione di default deriva il valore dalla
    /// [`SpellConfig`] e copre la stragrande maggioranza dei casi.
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

    /// Intervallo di accumulo tra un tick di channeling e il successivo.
    /// Rilevante solo per le spell channeling: il sistema centrale accumula il
    /// tempo trascorso e invoca [`cast`](Self::cast) solo quando supera questo
    /// intervallo. Default `0.25s`.
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
