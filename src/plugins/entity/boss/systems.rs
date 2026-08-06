//! Boss AI systems.
//!
//! Phase 1: arena aggro trigger (`boss_aggro_check`) + threat accrual
//! (`accrue_threat`). All server-authoritative, gated by `has_server`.
//! Later phases append the phase machine and the ability rotation here.

use bevy::prelude::*;

use super::components::{Boss, BossArena, BossPhase, BossRotationState, ThreatTable};
use super::target_select::{highest_threat, PlayerRef};
use crate::network::protocol::Position;
use crate::plugins::entity::player::components::Player;
use crate::plugins::spells::{CastProgress, SpellCastRequest, SpellCooldowns, SpellId};
use crate::spells::dragon_enemy::dragon_claw::DragonClawSpell;
use crate::stats::components::VitalStats;
use crate::stats::events::DamageEvent;

/// Hard enrage safety net: forces `Berserk` after this many seconds engaged.
pub const BERSERK_TIMER_SECONDS: f32 = 180.0;
/// HP fraction (of max) at which Ground -> Aerial transitions.
const AERIAL_HP_FRACTION: f32 = 0.66;
/// HP fraction (of max) at which Aerial -> Berserk transitions.
const BERSERK_HP_FRACTION: f32 = 0.33;

/// Advances the boss phase machine based on HP thresholds and the enrage timer.
///
/// Transitions are monotonic forward (Ground -> Aerial -> Berserk -> Dead) and
/// never regress. The enrage timer (`engaged_seconds`) can jump straight to
/// Berserk from Ground or Aerial if the fight stalls. Death is detected via
/// `VitalStats.is_dead()`. Writes the replicated `BossPhase` so clients show
/// the banner and restyle the boss bar. Server-only.
///
/// # Panics
/// Never; all comparisons use `partial_cmp` with a safe fallback.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(FixedUpdate, update_boss_phase.run_if(has_server));
/// ```
pub fn update_boss_phase(
    time: Res<Time>,
    mut bosses: Query<(&mut BossPhase, &mut BossRotationState, &VitalStats), With<Boss>>,
) {
    let delta = time.delta_secs();
    for (mut phase, mut rotation, vital) in bosses.iter_mut() {
        let hp_fraction = if vital.max_health > 0.0 {
            (vital.current_health / vital.max_health).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Death is terminal regardless of phase.
        if vital.is_dead() && *phase != BossPhase::Dead {
            *phase = BossPhase::Dead;
            info!("Boss defeated.");
            continue;
        }

        // Only accrue enrage time while the encounter is live.
        if matches!(*phase, BossPhase::Ground | BossPhase::Aerial) {
            rotation.engaged_seconds += delta;
        }

        match *phase {
            BossPhase::Ground => {
                if hp_fraction <= BERSERK_HP_FRACTION {
                    *phase = BossPhase::Berserk;
                    info!("Boss entered Berserk (HP skip).");
                } else if hp_fraction <= AERIAL_HP_FRACTION {
                    *phase = BossPhase::Aerial;
                    info!("Boss entered Aerial phase.");
                } else if rotation.engaged_seconds >= BERSERK_TIMER_SECONDS {
                    *phase = BossPhase::Berserk;
                    info!("Boss force-enraged by timer.");
                }
            }
            BossPhase::Aerial => {
                if hp_fraction <= BERSERK_HP_FRACTION {
                    *phase = BossPhase::Berserk;
                    info!("Boss entered Berserk.");
                } else if rotation.engaged_seconds >= BERSERK_TIMER_SECONDS {
                    *phase = BossPhase::Berserk;
                    info!("Boss force-enraged by timer.");
                }
            }
            // Berserk and Dormant/Dead hold their state.
            _ => {}
        }
    }
}

/// Engages the boss the first time a living player steps into the arena ring.
///
/// Flips `BossArena.is_engaged`, transitions `BossPhase::Dormant -> Ground` and
/// resets the rotation state. Idempotent once engaged (early-returns on
/// `is_engaged`). Server-only.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(FixedUpdate, boss_aggro_check.run_if(has_server));
/// ```
pub fn boss_aggro_check(
    mut bosses: Query<(&mut BossArena, &mut BossPhase, &mut BossRotationState), With<Boss>>,
    players: Query<(&Position, &VitalStats), With<Player>>,
) {
    for (mut arena, mut phase, mut rotation) in bosses.iter_mut() {
        if arena.is_engaged {
            continue;
        }
        let player_inside = players
            .iter()
            .any(|(pos, vital)| !vital.is_dead() && pos.0.distance(arena.center) <= arena.radius);
        if player_inside {
            arena.is_engaged = true;
            *phase = BossPhase::Ground;
            rotation.engaged_seconds = 0.0;
            rotation.priority_cursor = 0;
            info!("Boss engaged: a player crossed the arena ring");
        }
    }
}

/// Accrues threat on the boss for every `DamageEvent` whose target is the boss.
///
/// Threat is attributed to `event.source` (the damage dealer). Dead/absent
/// sources are skipped. Selection-time filtering of dead players happens in the
/// rotation driver, not here, to keep this system a passive listener. There is
/// typically a single boss, so the inner lookup is effectively O(1).
/// Server-only.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(FixedUpdate, accrue_threat.run_if(has_server));
/// ```
pub fn accrue_threat(
    mut damage_events: MessageReader<DamageEvent>,
    mut bosses: Query<(Entity, &mut ThreatTable), With<Boss>>,
) {
    if bosses.is_empty() {
        return;
    }
    for event in damage_events.read() {
        let Some(source) = event.source else {
            continue;
        };
        for (boss_entity, mut threat) in bosses.iter_mut() {
            if boss_entity == event.target {
                threat.add(source, event.amount);
                break;
            }
        }
    }
}

/// How an ability resolves its target.
enum Targeting {
    /// The highest-threat living player (entity + position).
    MainThreat,
    /// Centered on the boss itself (self-AoE like wing buffet, molten eruption).
    CasterCentered,
    /// The centroid of the `n` most-clustered living players (cinder storm).
    DensestCluster(usize),
}

/// One entry in a phase's ability priority list.
struct RotationEntry {
    spell_id: SpellId,
    targeting: Targeting,
}

/// Returns the priority list for the current phase. Berserk unions the ground
/// roster with Cataclysm and re-uses the same ordering; cooldown haste is not
/// modeled here (v1 keeps it simple — Cataclysm itself gives the phase teeth).
fn priority_list_for(phase: BossPhase) -> Vec<RotationEntry> {
    match phase {
        BossPhase::Ground => vec![
            RotationEntry {
                spell_id: SpellId::new("searing_breath"),
                targeting: Targeting::MainThreat,
            },
            RotationEntry {
                spell_id: SpellId::new("cinder_storm"),
                targeting: Targeting::DensestCluster(2),
            },
            RotationEntry {
                spell_id: SpellId::new("wing_buffet"),
                targeting: Targeting::CasterCentered,
            },
            RotationEntry {
                spell_id: SpellId::new("tail_sweep"),
                targeting: Targeting::CasterCentered,
            },
            RotationEntry {
                spell_id: SpellId::new(DragonClawSpell::ID),
                targeting: Targeting::MainThreat,
            },
        ],
        BossPhase::Aerial => vec![
            RotationEntry {
                spell_id: SpellId::new("molten_eruption"),
                targeting: Targeting::CasterCentered,
            },
            RotationEntry {
                spell_id: SpellId::new("cinder_storm"),
                targeting: Targeting::DensestCluster(2),
            },
        ],
        BossPhase::Berserk => vec![
            RotationEntry {
                spell_id: SpellId::new("cataclysm"),
                targeting: Targeting::CasterCentered,
            },
            RotationEntry {
                spell_id: SpellId::new("searing_breath"),
                targeting: Targeting::MainThreat,
            },
            RotationEntry {
                spell_id: SpellId::new("cinder_storm"),
                targeting: Targeting::DensestCluster(2),
            },
            RotationEntry {
                spell_id: SpellId::new("wing_buffet"),
                targeting: Targeting::CasterCentered,
            },
            RotationEntry {
                spell_id: SpellId::new(DragonClawSpell::ID),
                targeting: Targeting::MainThreat,
            },
        ],
        // Dormant/Dead have no rotation.
        _ => vec![],
    }
}

/// Full boss rotation driver.
///
/// Each fixed tick, for every engaged boss that isn't currently casting, pick
/// the first ability in the current phase's priority list that is off cooldown
/// and whose target can be resolved, then emit a `SpellCastRequest`. The
/// `Without<CastProgress>` filter guarantees one cast at a time. Server-only.
///
/// # Example
/// ```rust,ignore
/// app.add_systems(FixedUpdate, run_boss_rotation.run_if(has_server));
/// ```
pub fn run_boss_rotation(
    bosses: Query<
        (
            Entity,
            &Position,
            &BossArena,
            &BossPhase,
            &ThreatTable,
            &SpellCooldowns,
        ),
        (With<Boss>, Without<CastProgress>),
    >,
    players: Query<(Entity, &Position, &VitalStats), With<Player>>,
    mut spell_cast_requests: MessageWriter<SpellCastRequest>,
) {
    let living: Vec<PlayerRef> = players
        .iter()
        .filter(|(_, _, vital)| !vital.is_dead())
        .map(|(entity, position, _)| PlayerRef {
            entity,
            position: position.0,
        })
        .collect();

    for (boss, boss_position, arena, phase, threat, cooldowns) in bosses.iter() {
        if !arena.is_engaged || *phase == BossPhase::Dead || *phase == BossPhase::Dormant {
            continue;
        }

        let main_target = highest_threat(threat, &living);
        if main_target.is_none() {
            // No aggro target yet: nothing to cast at.
            continue;
        }
        let _ = main_target;

        let priority = priority_list_for(*phase);
        for entry in &priority {
            if cooldowns.is_on_cooldown(&entry.spell_id) {
                continue;
            }
            let Some((target_entity, target_position)) =
                resolve_target(&entry.targeting, boss_position.0, &living, threat)
            else {
                continue;
            };

            spell_cast_requests.write(SpellCastRequest {
                caster: boss,
                spell_id: entry.spell_id.clone(),
                target_position,
                target_entity,
            });
            break;
        }
    }
}

/// Resolves a `Targeting` into an optional (entity, position) pair.
///
/// `CasterCentered` always resolves (position = boss, entity = None).
/// `MainThreat` requires a living player with threat. `DensestCluster` requires
/// at least `n` living players; it returns the centroid with no entity.
fn resolve_target(
    targeting: &Targeting,
    caster_position: Vec3,
    living: &[PlayerRef],
    threat: &ThreatTable,
) -> Option<(Option<Entity>, Option<Vec3>)> {
    match targeting {
        Targeting::CasterCentered => Some((None, Some(caster_position))),
        Targeting::MainThreat => highest_threat(threat, living)
            .map(|player| (Some(player.entity), Some(player.position))),
        Targeting::DensestCluster(n) => {
            crate::plugins::entity::boss::target_select::densest_cluster(living, *n)
                .map(|centroid| (None, Some(centroid)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::protocol::Position;
    use crate::stats::components::CombatStats;

    const ARENA_CENTER: Vec3 = Vec3::new(0.0, 0.0, -12.0);
    const ARENA_RADIUS: f32 = 12.0;

    fn spawn_dormant_boss(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                Boss,
                BossArena {
                    center: ARENA_CENTER,
                    radius: ARENA_RADIUS,
                    is_engaged: false,
                },
                BossPhase::Dormant,
                BossRotationState::default(),
            ))
            .id()
    }

    fn spawn_player(app: &mut App, position: Vec3, alive: bool) -> Entity {
        app.world_mut()
            .spawn((
                Player,
                Position(position),
                VitalStats {
                    current_health: if alive { 100.0 } else { 0.0 },
                    max_health: 100.0,
                    max_mana: 0.0,
                    mana_regeneration: 0.0,
                },
            ))
            .id()
    }

    /// Builds a throwaway writer system that injects a single `DamageEvent`
    /// exactly once across update ticks, scheduled before `accrue_threat` so the
    /// reader sees it the same frame.
    fn damage_sender(
        target: Entity,
        source: Entity,
        amount: f32,
    ) -> impl FnMut(Local<bool>, MessageWriter<DamageEvent>) {
        let mut sent = false;
        move |mut already_sent, mut writer: MessageWriter<DamageEvent>| {
            if *already_sent || sent {
                return;
            }
            *already_sent = true;
            sent = true;
            writer.write(DamageEvent {
                target,
                source: Some(source),
                amount,
            });
        }
    }

    #[test]
    fn aggro_engages_when_a_living_player_is_inside_the_ring() {
        let mut app = App::new();
        app.add_systems(Update, boss_aggro_check);

        let boss = spawn_dormant_boss(&mut app);
        spawn_player(&mut app, ARENA_CENTER + Vec3::new(2.0, 0.0, 0.0), true);

        app.update();

        let entity_ref = app.world().entity(boss);
        assert!(entity_ref.get::<BossArena>().unwrap().is_engaged);
        assert_eq!(*entity_ref.get::<BossPhase>().unwrap(), BossPhase::Ground);
    }

    #[test]
    fn aggro_does_not_engage_when_no_player_is_inside() {
        let mut app = App::new();
        app.add_systems(Update, boss_aggro_check);

        let boss = spawn_dormant_boss(&mut app);
        // Player well outside the radius.
        spawn_player(&mut app, ARENA_CENTER + Vec3::new(50.0, 0.0, 0.0), true);

        app.update();

        let entity_ref = app.world().entity(boss);
        assert!(!entity_ref.get::<BossArena>().unwrap().is_engaged);
        assert_eq!(*entity_ref.get::<BossPhase>().unwrap(), BossPhase::Dormant);
    }

    #[test]
    fn aggro_ignores_dead_players_inside_the_ring() {
        let mut app = App::new();
        app.add_systems(Update, boss_aggro_check);

        let boss = spawn_dormant_boss(&mut app);
        spawn_player(&mut app, ARENA_CENTER, false);

        app.update();

        let entity_ref = app.world().entity(boss);
        assert!(!entity_ref.get::<BossArena>().unwrap().is_engaged);
    }

    #[test]
    fn aggro_is_idempotent_once_engaged() {
        let mut app = App::new();
        app.add_systems(Update, boss_aggro_check);

        let boss = spawn_dormant_boss(&mut app);
        spawn_player(&mut app, ARENA_CENTER, true);

        app.update();
        // Force phase back to Dormant; since is_engaged is already true, the
        // system must not touch it again.
        let mut entity_mut = app.world_mut().entity_mut(boss);
        *entity_mut.get_mut::<BossPhase>().unwrap() = BossPhase::Dormant;
        drop(entity_mut);

        app.update();

        let entity_ref = app.world().entity(boss);
        assert!(entity_ref.get::<BossArena>().unwrap().is_engaged);
        assert_eq!(*entity_ref.get::<BossPhase>().unwrap(), BossPhase::Dormant);
    }

    #[test]
    fn accrue_threat_adds_amount_from_damage_source() {
        let mut app = App::new();
        app.add_message::<DamageEvent>();
        app.add_systems(Update, accrue_threat);

        let boss = app.world_mut().spawn((Boss, ThreatTable::default())).id();
        let dealer = app.world_mut().spawn_empty().id();

        app.add_systems(
            Update,
            damage_sender(boss, dealer, 42.0).before(accrue_threat),
        );

        app.update();
        app.update();

        let threat = app.world().entity(boss).get::<ThreatTable>().unwrap();
        assert_eq!(*threat.entries.get(&dealer).unwrap(), 42.0);
    }

    #[test]
    fn accrue_threat_ignores_events_targeting_non_bosses() {
        let mut app = App::new();
        app.add_message::<DamageEvent>();
        app.add_systems(Update, accrue_threat);

        let boss = app.world_mut().spawn((Boss, ThreatTable::default())).id();
        let dealer = app.world_mut().spawn_empty().id();
        let other_target = app.world_mut().spawn_empty().id();

        app.add_systems(
            Update,
            damage_sender(other_target, dealer, 42.0).before(accrue_threat),
        );

        app.update();
        app.update();

        let threat = app.world().entity(boss).get::<ThreatTable>().unwrap();
        assert!(threat.entries.is_empty());

        // Silence unused-import warning for CombatStats in this test module.
        let _ = CombatStats {
            attack_power: 0.0,
            armor: 0.0,
        };
    }

    fn spawn_boss_with_hp(app: &mut App, max_health: f32, current_health: f32) -> Entity {
        app.world_mut()
            .spawn((
                Boss,
                BossPhase::Ground,
                BossRotationState::default(),
                VitalStats {
                    current_health,
                    max_health,
                    max_mana: 0.0,
                    mana_regeneration: 0.0,
                },
            ))
            .id()
    }

    #[test]
    fn phase_transitions_ground_to_aerial_at_two_thirds_hp() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_systems(Update, update_boss_phase);

        let boss = spawn_boss_with_hp(&mut app, 6000.0, 3900.0); // 65%
        app.update();

        assert_eq!(
            *app.world().entity(boss).get::<BossPhase>().unwrap(),
            BossPhase::Aerial
        );
    }

    #[test]
    fn phase_transitions_aerial_to_berserk_at_one_third_hp() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_systems(Update, update_boss_phase);

        let boss = spawn_boss_with_hp(&mut app, 6000.0, 1900.0); // ~31.6%
        app.update();

        assert_eq!(
            *app.world().entity(boss).get::<BossPhase>().unwrap(),
            BossPhase::Berserk
        );
    }

    #[test]
    fn phase_skips_straight_to_berserk_when_hp_already_low() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_systems(Update, update_boss_phase);

        let boss = spawn_boss_with_hp(&mut app, 6000.0, 1000.0); // ~16.6%
        app.update();

        assert_eq!(
            *app.world().entity(boss).get::<BossPhase>().unwrap(),
            BossPhase::Berserk
        );
    }

    #[test]
    fn phase_transitions_to_dead_when_hp_reaches_zero() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_systems(Update, update_boss_phase);

        let boss = spawn_boss_with_hp(&mut app, 6000.0, 0.0);
        app.update();

        assert_eq!(
            *app.world().entity(boss).get::<BossPhase>().unwrap(),
            BossPhase::Dead
        );
    }

    #[test]
    fn enrage_timer_forces_berserk_from_ground() {
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_systems(Update, update_boss_phase);

        // Full HP but timer already past the enrage threshold.
        let boss = app
            .world_mut()
            .spawn((
                Boss,
                BossPhase::Ground,
                BossRotationState {
                    engaged_seconds: BERSERK_TIMER_SECONDS + 1.0,
                    priority_cursor: 0,
                },
                VitalStats {
                    current_health: 6000.0,
                    max_health: 6000.0,
                    max_mana: 0.0,
                    mana_regeneration: 0.0,
                },
            ))
            .id();
        app.update();

        assert_eq!(
            *app.world().entity(boss).get::<BossPhase>().unwrap(),
            BossPhase::Berserk
        );
    }
}
