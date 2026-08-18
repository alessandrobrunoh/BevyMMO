//! What the mobs do: aggroing, chasing, hitting, and — for the dragon —
//! phases, threat and an ability rotation.
//!
//! Ported from `gameplay/entity/enemy/systems.rs` (`enemy_chase`,
//! `enemy_auto_cast_attack`), `gameplay/entity/boss/systems.rs` (the phase
//! machine, arena aggro, the rotation driver) and
//! `gameplay/entity/boss/target_select.rs` (the pure selection helpers, which
//! are re-stated here because they were written against `bevy::Entity`).
//!
//! # Where the death transition went
//!
//! `gameplay/entity/systems.rs::mark_dead_entities` is *not* here. It landed as
//! `sim::combat::reap_the_dead`, which runs at the end of `sim::combat::step` —
//! the step immediately before this one — precisely so the AI sees corpses as
//! corpses. Marking the dead again here would give one transition two writers,
//! so this module instead treats `game_entity.state` as authoritative, which is
//! exactly what the Bevy enemy and boss systems did (`if state.is_dead() {
//! continue; }`). The dependency runs the other way too: were the sweep ever
//! moved out of `combat::step`, or reordered after this one, the AI would drive
//! corpses for a tick.
//!
//! # Finding targets without an ECS
//!
//! The Bevy version answered "who is near me" with a `Query<&Position,
//! With<Player>>` and a `min_by` over *every* player, once per mob per tick.
//! That is `O(mobs × players)` and it was affordable only because Bevy handed
//! the system a packed array. Repeating it here would mean a full table scan of
//! `game_entity` per mob per tick, which does not survive a populated map.
//!
//! Instead every "who is within R of P" question goes through
//! [`living_players_near`], which walks the `cell_x`/`cell_z` btree index on
//! `game_entity`: the query rectangle covers `(2R / GRID_CELL_SIZE) + 2` cells
//! per axis, and the index is scanned one `cell_x` column at a time with a
//! range over `cell_z` — the only shape a multi-column btree supports, since
//! every column before the last must be an exact value. With
//! `GRID_CELL_SIZE = 16`, an enemy's 10-unit aggro check touches at most 2×2
//! cells and the boss's 12-unit arena at most 3×3, whatever the population.
//!
//! One full scan of `game_entity` per tick remains, in [`collect_actors`]:
//! there is no index on `kind`, so the mobs have to be found by looking. It is
//! one pass for the whole tick rather than one per mob.

use bevymmo_domain::content::spells::fireball::FireballSpell;
use bevymmo_domain::entity::boss::components::{Boss, BossPhase, BossRotationState, BossSpellbook};
use bevymmo_domain::entity::enemy::components::AggroRange;
use bevymmo_domain::movement;
use bevymmo_domain::spells::context::ChannelMovementPolicy as SpellChannelMovementPolicy;
use bevymmo_domain::spells::{CastKind, Spell, SpellId};
use glam::Vec3;
use spacetimedb::{ReducerContext, Table};

use crate::rows::Vec3Row;
use crate::sim::targets;
use crate::sim::{crowd_control, spells};
use crate::tables::{
    boss_state, cast_state, entity_stats, game_entity, grid_cell, threat, BossPhaseRow, BossState,
    CastState, EntityKindRow, EntityStateRow, GameEntity, Threat,
};

/// Distance a melee attacker stops at, so it fights the target instead of
/// standing inside it. From `boss/systems.rs::MELEE_REACH`.
const MELEE_REACH: f32 = 3.0;

/// How close to its spawn a leashing enemy has to get before it stops walking.
///
/// Purely so it does not twitch between "one centimetre out" and "home".
const HOME_ARRIVAL_EPSILON: f32 = 0.25;

/// Hard enrage safety net: the boss is forced to `Enraged` after this many
/// seconds engaged. From `boss/systems.rs::BERSERK_TIMER_SECONDS`.
const BERSERK_TIMER_SECONDS: f32 = 180.0;

/// HP fraction at which the boss leaves phase one.
const AERIAL_HP_FRACTION: f32 = 0.66;

/// HP fraction at which the boss enrages.
const BERSERK_HP_FRACTION: f32 = 0.33;

/// Ceiling on the radius of a spatial query, in world units.
///
/// A radius is data — an arena row could be seeded with a nonsense value — and
/// the cell loop is quadratic in it. 128 units is eight cells per axis either
/// way, far past anything the encounters use, and bounds the worst tick.
const MAX_SPATIAL_QUERY_RADIUS: f32 = 128.0;

/// The mobs this tick has to drive, gathered in a single pass.
struct Actors {
    enemies: Vec<GameEntity>,
    bosses: Vec<GameEntity>,
}

/// A living player as the selection helpers want it: id and position.
///
/// The Bevy shape carried a `bevy::Entity`; here it is the `game_entity` key.
struct PlayerRef {
    entity: u64,
    position: Vec3,
}

pub fn step(ctx: &ReducerContext, dt: f32) {
    let actors = collect_actors(ctx);
    for enemy in actors.enemies {
        step_enemy(ctx, enemy);
    }
    for boss in actors.bosses {
        step_boss(ctx, boss, dt);
    }
}

/// The one full scan of `game_entity` this step takes, splitting out the mobs
/// that have AI.
///
/// Collected into `Vec`s rather than driven inline because everything below
/// writes to `game_entity` — chasing, facing, dying targets — and a tick may
/// not mutate a table it is iterating.
fn collect_actors(ctx: &ReducerContext) -> Actors {
    let mut enemies = Vec::new();
    let mut bosses = Vec::new();

    for entity in ctx.db.game_entity().iter() {
        if entity.state == EntityStateRow::Dead {
            continue;
        }
        match entity.kind {
            EntityKindRow::Enemy => enemies.push(entity),
            EntityKindRow::Boss => bosses.push(entity),
            // Players drive themselves; dummies and NPCs have no AI.
            EntityKindRow::Player | EntityKindRow::Dummy | EntityKindRow::Npc => {}
        }
    }

    Actors { enemies, bosses }
}

// ---------------------------------------------------------------------------
// Enemies
// ---------------------------------------------------------------------------

/// Aggro range shared by every enemy.
///
/// The Bevy server carried it per-entity in an `AggroRange` component;
/// `game_entity` has no column for it, so the domain default stands in for all
/// of them. See the port report: a per-mob range needs a schema field.
fn enemy_aggro_range() -> f32 {
    AggroRange::default().0
}

/// One enemy's turn: pick a target, close to melee, swing, or walk home.
///
/// Two departures from `enemy_chase`/`enemy_auto_cast_attack`:
///
/// - Bevy moved the enemy by writing `position += direction * speed` every
///   tick, where `speed` was implicitly per-tick at a fixed 60 Hz. Here the AI
///   only chooses a `move_target` and `sim::movement::step` walks it in units
///   per second, so the enemy covers ground at the same rate whatever the tick
///   length is, and arrives through the same code path a player does.
/// - Bevy had no leash: an enemy that lost its target stopped wherever it
///   stood, which over a session drags every mob out of its camp. This one
///   walks back to its `spawn_point`, the field the schema already carries for
///   exactly that.
fn step_enemy(ctx: &ReducerContext, enemy: GameEntity) {
    let entity_id = enemy.entity_id;
    let position = Vec3::from(enemy.position);
    let target = nearest_living_player(ctx, position, enemy_aggro_range());

    let (look, move_target) = match &target {
        Some(target) => {
            let look = movement::look_direction(position, target.position)
                .map(Vec3Row::from)
                .unwrap_or(enemy.look);
            // Aim one melee reach short of the target: walking all the way onto
            // it is what let the Bevy enemies climb into the player model.
            let offset = target.position - position;
            let horizontal = Vec3::new(offset.x, 0.0, offset.z);
            let distance = horizontal.length();
            let move_target = if distance > MELEE_REACH {
                let direction = horizontal / distance;
                Some(Vec3Row::from(
                    position + direction * (distance - MELEE_REACH),
                ))
            } else {
                // In reach: hold still and fight.
                None
            };
            (look, move_target)
        }
        None => {
            let home = Vec3::from(enemy.spawn_point);
            let move_target = if position.distance(home) > HOME_ARRIVAL_EPSILON {
                Some(enemy.spawn_point)
            } else {
                None
            };
            (enemy.look, move_target)
        }
    };

    let move_target = gate_movement(ctx, entity_id, move_target);
    let enemy = write_pose(ctx, enemy, look, move_target);

    if let Some(target) = target {
        try_attack(ctx, &enemy, &target);
    }
}

/// Requests the basic attack when the target is genuinely reachable.
///
/// `enemy_auto_cast_attack` fired at *aggro* range — ten units — even though
/// `attack` only lands within its three-unit radius, so every enemy in the zone
/// burned its cooldown swinging at air the whole way in. The gate here is the
/// spell's own radius.
fn try_attack(ctx: &ReducerContext, enemy: &GameEntity, target: &PlayerRef) {
    let position = Vec3::from(enemy.position);
    if position.distance(target.position) > FireballSpell.config().cast_range {
        return;
    }
    if !can_start_cast(ctx, enemy.entity_id) {
        return;
    }
    if spells::is_on_cooldown(ctx, enemy.entity_id, FireballSpell::ID) {
        return;
    }
    request_cast(
        ctx,
        enemy,
        &SpellId::new(FireballSpell::ID),
        Some(target.entity),
        Some(target.position),
    );
}

// ---------------------------------------------------------------------------
// Boss
// ---------------------------------------------------------------------------

/// How an ability picks what it points at. From `boss/systems.rs::Targeting`.
enum Targeting {
    /// Highest threat, or the nearest player before anyone has any.
    MainThreat,
    /// The farthest living player — Enraged punishing the backline.
    Farthest,
    /// Centred on the boss itself.
    CasterCentered,
    /// The centroid of the `n` most tightly packed living players.
    DensestCluster(usize),
}

/// One entry in a phase's priority list.
struct RotationEntry {
    spell_id: &'static str,
    targeting: Targeting,
}

/// Phase one's priority list: grounded melee and breath.
static GROUND_ROTATION: &[RotationEntry] = &[
    RotationEntry {
        spell_id: "searing_breath",
        targeting: Targeting::MainThreat,
    },
    RotationEntry {
        spell_id: "cinder_storm",
        targeting: Targeting::DensestCluster(2),
    },
    RotationEntry {
        spell_id: "wing_buffet",
        targeting: Targeting::CasterCentered,
    },
    RotationEntry {
        spell_id: "tail_sweep",
        targeting: Targeting::CasterCentered,
    },
    RotationEntry {
        spell_id: "dragon_claw",
        targeting: Targeting::MainThreat,
    },
];

/// Phase two: airborne, so only the arena-wide patterns.
static AERIAL_ROTATION: &[RotationEntry] = &[
    RotationEntry {
        spell_id: "molten_eruption",
        targeting: Targeting::CasterCentered,
    },
    RotationEntry {
        spell_id: "cinder_storm",
        targeting: Targeting::DensestCluster(2),
    },
];

/// Enraged: the ground roster with Cataclysm on top and the breath aimed at
/// whoever thought distance would save them.
static BERSERK_ROTATION: &[RotationEntry] = &[
    RotationEntry {
        spell_id: "cataclysm",
        targeting: Targeting::CasterCentered,
    },
    RotationEntry {
        spell_id: "searing_breath",
        targeting: Targeting::Farthest,
    },
    RotationEntry {
        spell_id: "cinder_storm",
        targeting: Targeting::DensestCluster(2),
    },
    RotationEntry {
        spell_id: "wing_buffet",
        targeting: Targeting::CasterCentered,
    },
    RotationEntry {
        spell_id: "dragon_claw",
        targeting: Targeting::MainThreat,
    },
];

/// The priority list for a phase.
///
/// `priority_list_for` built a fresh `Vec<RotationEntry>` every tick for every
/// boss; the lists are constant, so they are statics here and the rotation
/// driver allocates nothing.
fn priority_list_for(phase: BossPhase) -> &'static [RotationEntry] {
    match phase {
        BossPhase::Ground => GROUND_ROTATION,
        BossPhase::Aerial => AERIAL_ROTATION,
        BossPhase::Berserk => BERSERK_ROTATION,
        // Dormant and Dead have no rotation.
        BossPhase::Dormant | BossPhase::Dead => &[],
    }
}

/// One boss's turn: engage, advance the phase, chase, cast.
///
/// `boss_aggro_check`, `update_boss_phase`, `boss_chase` and
/// `run_boss_rotation` were four systems over the same handful of components.
/// Here they are four steps over one `boss_state` row, written back once at the
/// end — four round trips through the same row would be four times the work for
/// the same answer.
fn step_boss(ctx: &ReducerContext, boss: GameEntity, dt: f32) {
    let entity_id = boss.entity_id;
    let Some(state) = ctx.db.boss_state().entity_id().find(entity_id) else {
        // A boss entity with no arena row is content that has not been seeded
        // yet. Nothing to drive, and nothing worth logging every tick.
        return;
    };

    let arena_center = Vec3::from(state.arena_center);
    // A missing or nonsense radius falls back to the encounter's own constant
    // rather than to zero, which would make the arena impossible to enter.
    let arena_radius = if state.arena_radius > 0.0 {
        state.arena_radius
    } else {
        Boss::ARENA_RADIUS
    };

    // Candidates are the living players *inside the arena*. Bevy considered
    // every player in the world, which is free with an ECS query and is not
    // free here — and a player who has left the ring has left the fight, so the
    // narrower set is also the more correct one. It is what makes the whole
    // encounter cost a 3×3 cell scan per tick.
    let living = living_players_near(ctx, arena_center, arena_radius);

    let mut phase = phase_from_row(state.phase);
    let mut is_engaged = state.is_engaged;
    // The domain's own scheduler state, borrowed for the tick so the phase
    // rules below read exactly as `update_boss_phase` did.
    let mut rotation = BossRotationState {
        engaged_seconds: state.engaged_seconds,
        priority_cursor: state.rotation_cursor as usize,
    };

    if !is_engaged {
        if living.is_empty() {
            return;
        }
        is_engaged = true;
        phase = BossPhase::Ground;
        rotation.engaged_seconds = 0.0;
        rotation.priority_cursor = 0;
        log::info!("Boss {entity_id} engaged: a player crossed the arena ring");
    }

    phase = advance_phase(ctx, entity_id, phase, &mut rotation, dt);

    let boss_position = Vec3::from(boss.position);
    match main_target(ctx, entity_id, &living, boss_position) {
        Some(main) => {
            // `chase` hands the row back because it may have rewritten `look`,
            // and a breath fired from a stale facing points at where the target
            // used to be.
            let boss = chase(ctx, boss, phase, main);
            run_rotation(ctx, &boss, phase, &living, main, &mut rotation);
        }
        None => {
            // Engaged with nobody in the ring: the fight is still running (the
            // enrage timer keeps ticking) but there is nothing to aim at.
        }
    }

    ctx.db.boss_state().entity_id().update(BossState {
        phase: phase_to_row(phase),
        is_engaged,
        engaged_seconds: rotation.engaged_seconds,
        rotation_cursor: rotation.priority_cursor as u32,
        ..state
    });
}

/// The phase machine: HP thresholds first, enrage timer as the safety net.
///
/// Transitions are monotonic forward and never regress, as in
/// `update_boss_phase`. Death is *not* handled here: `BossPhaseRow` has no
/// `Dead` variant, so a defeated boss is one whose `game_entity.state` is
/// `Dead`, and `collect_actors` filtered it out before this ran. The phase
/// column keeps the phase the boss died in, which is the more useful thing for
/// a client to show over the corpse anyway.
fn advance_phase(
    ctx: &ReducerContext,
    entity_id: u64,
    phase: BossPhase,
    rotation: &mut BossRotationState,
    dt: f32,
) -> BossPhase {
    let hp_fraction = health_fraction(ctx, entity_id);

    // Only accrue enrage time while the encounter is live.
    if matches!(phase, BossPhase::Ground | BossPhase::Aerial) {
        rotation.engaged_seconds += dt;
    }

    match phase {
        BossPhase::Ground => {
            if hp_fraction <= BERSERK_HP_FRACTION {
                log::info!("Boss {entity_id} entered Berserk (HP skip)");
                BossPhase::Berserk
            } else if hp_fraction <= AERIAL_HP_FRACTION {
                log::info!("Boss {entity_id} entered the aerial phase");
                BossPhase::Aerial
            } else if rotation.engaged_seconds >= BERSERK_TIMER_SECONDS {
                log::info!("Boss {entity_id} force-enraged by timer");
                BossPhase::Berserk
            } else {
                phase
            }
        }
        BossPhase::Aerial => {
            if hp_fraction <= BERSERK_HP_FRACTION {
                log::info!("Boss {entity_id} entered Berserk");
                BossPhase::Berserk
            } else if rotation.engaged_seconds >= BERSERK_TIMER_SECONDS {
                log::info!("Boss {entity_id} force-enraged by timer");
                BossPhase::Berserk
            } else {
                phase
            }
        }
        // Berserk holds, Dormant is unreachable once engaged, Dead is terminal.
        BossPhase::Berserk | BossPhase::Dormant | BossPhase::Dead => phase,
    }
}

/// Walks the boss towards its main target, stopping at melee reach, and returns
/// the row as it now stands.
///
/// The aerial phase does not chase: the dragon is flying. Facing is updated in
/// every phase, because the cone abilities — searing breath, tail sweep — are
/// aimed by `look` at the moment they fire.
fn chase(
    ctx: &ReducerContext,
    boss: GameEntity,
    phase: BossPhase,
    target: &PlayerRef,
) -> GameEntity {
    let position = Vec3::from(boss.position);
    let offset = target.position - position;
    let horizontal = Vec3::new(offset.x, 0.0, offset.z);
    let distance = horizontal.length();
    if distance < 0.001 {
        return boss;
    }
    let direction = horizontal / distance;
    let look = Vec3Row::from(direction);

    let grounded = matches!(phase, BossPhase::Ground | BossPhase::Berserk);
    let move_target = if grounded && distance > MELEE_REACH {
        Some(Vec3Row::from(
            position + direction * (distance - MELEE_REACH),
        ))
    } else {
        None
    };
    let move_target = gate_movement(ctx, boss.entity_id, move_target);

    write_pose(ctx, boss, look, move_target)
}

/// The rotation driver: first ability in the phase's priority list that is off
/// cooldown and whose target resolves.
///
/// Strict priority, as in `run_boss_rotation` — the order *is* the design of
/// the fight, and the cooldowns are what make it rotate. `rotation_cursor`
/// therefore records which entry was chosen rather than steering the scan;
/// treating it as a round-robin start would quietly reorder every phase.
fn run_rotation(
    ctx: &ReducerContext,
    boss: &GameEntity,
    phase: BossPhase,
    living: &[PlayerRef],
    main: &PlayerRef,
    rotation: &mut BossRotationState,
) {
    let entity_id = boss.entity_id;
    if !can_start_cast(ctx, entity_id) {
        return;
    }
    // The boss's own spellbook, which is what let the dragon cycle more than
    // the three slots a player hotbar holds. Checked here so a rotation entry
    // that drifts away from `Boss::SPELLS` fails loudly at the source, rather
    // than as a lookup miss deep inside the cast pipeline.
    let spellbook = BossSpellbook {
        spells: Boss::SPELLS.iter().copied().map(SpellId::new).collect(),
    };
    let boss_position = Vec3::from(boss.position);

    for (index, entry) in priority_list_for(phase).iter().enumerate() {
        let spell_id = SpellId::new(entry.spell_id);
        if !spellbook.contains(&spell_id) {
            log::warn!(
                "boss rotation lists {:?}, which is not in Boss::SPELLS",
                entry.spell_id
            );
            continue;
        }
        if spells::is_on_cooldown(ctx, entity_id, entry.spell_id) {
            continue;
        }
        let Some((target_entity, target_position)) =
            resolve_target(&entry.targeting, boss_position, living, main)
        else {
            continue;
        };
        request_cast(ctx, boss, &spell_id, target_entity, target_position);
        rotation.priority_cursor = index;
        break;
    }
}

/// Turns a `Targeting` into an (entity, position) pair, or `None` when the
/// ability has nothing to point at this tick.
fn resolve_target(
    targeting: &Targeting,
    caster_position: Vec3,
    living: &[PlayerRef],
    main: &PlayerRef,
) -> Option<(Option<u64>, Option<Vec3>)> {
    match targeting {
        Targeting::CasterCentered => Some((None, Some(caster_position))),
        Targeting::MainThreat => Some((Some(main.entity), Some(main.position))),
        Targeting::Farthest => {
            farthest_target(living, caster_position).map(|p| (Some(p.entity), Some(p.position)))
        }
        Targeting::DensestCluster(n) => densest_cluster(living, *n).map(|c| (None, Some(c))),
    }
}

// ---------------------------------------------------------------------------
// Threat
// ---------------------------------------------------------------------------

/// Records `amount` of threat on `target` from `source`, if `target` is a boss.
///
/// The entry point `accrue_threat` was: it read every `DamageEvent` and added
/// to the `ThreatTable` of the boss that was hit. `damage_event` here is an
/// *event* table — delivered to subscribers, never read back — so the accrual
/// cannot listen after the fact. `sim::combat::apply_damage` should call this
/// as it applies damage. Until it does, the boss falls back to nearest-player
/// targeting, which is the same fallback the Bevy version used before the first
/// hit landed, so the encounter works and only the aggro *ordering* is missing.
///
/// The `boss_state` lookup is a primary-key hit, so a call for a non-boss
/// target costs one point read and returns.
pub fn accrue_threat(ctx: &ReducerContext, target: u64, source: u64, amount: f32) {
    if amount <= 0.0 || ctx.db.boss_state().entity_id().find(target).is_none() {
        return;
    }
    let existing = ctx
        .db
        .threat()
        .by_boss()
        .filter(target)
        .find(|row| row.target_entity == source);

    match existing {
        Some(row) => {
            ctx.db.threat().id().update(Threat {
                amount: row.amount + amount,
                ..row
            });
        }
        None => {
            ctx.db.threat().insert(Threat {
                // Zero asks the sequence for an id.
                id: 0,
                boss_entity: target,
                target_entity: source,
                amount,
            });
        }
    }
}

/// The boss's primary target: highest threat, or the nearest player when nobody
/// has any yet.
///
/// The fallback is what makes the dragon start swinging the moment the ring is
/// crossed, instead of standing there until someone hits it first.
///
/// `ThreatTable` is not loaded here even though the rule is its own: the
/// `threat` table *is* that map, already indexed by boss, so rebuilding a
/// `HashMap` from it every tick would allocate for nothing — and would pull in
/// the standard hasher's platform randomness, which this module does not have.
fn main_target<'a>(
    ctx: &ReducerContext,
    boss_entity: u64,
    living: &'a [PlayerRef],
    origin: Vec3,
) -> Option<&'a PlayerRef> {
    let mut best: Option<(&PlayerRef, f32)> = None;
    for row in ctx.db.threat().by_boss().filter(boss_entity) {
        // Threat from someone dead or out of the arena does not count; the row
        // is kept, so the meters still read right when they come back.
        let Some(player) = living.iter().find(|p| p.entity == row.target_entity) else {
            continue;
        };
        if best.is_none_or(|(_, amount)| row.amount > amount) {
            best = Some((player, row.amount));
        }
    }
    best.map(|(player, _)| player)
        .or_else(|| nearest_target(living, origin))
}

// ---------------------------------------------------------------------------
// Target selection (ported from boss/target_select.rs)
// ---------------------------------------------------------------------------

/// The nearest of `players` to `origin`.
fn nearest_target(players: &[PlayerRef], origin: Vec3) -> Option<&PlayerRef> {
    players.iter().min_by(|left, right| {
        left.position
            .distance_squared(origin)
            .total_cmp(&right.position.distance_squared(origin))
    })
}

/// The farthest of `players` from `origin`.
fn farthest_target(players: &[PlayerRef], origin: Vec3) -> Option<&PlayerRef> {
    players.iter().max_by(|left, right| {
        left.position
            .distance_squared(origin)
            .total_cmp(&right.position.distance_squared(origin))
    })
}

/// The most players a cluster search will look at.
///
/// The search is `O(C(p, n))`. The arena bounds `p` in practice, but the bound
/// is content, not code, so it is stated: past this many candidates the search
/// takes the first `CLUSTER_CANDIDATE_LIMIT` and accepts a slightly worse
/// circle over a tick that grows like a binomial.
const CLUSTER_CANDIDATE_LIMIT: usize = 12;

/// The centroid of the `n` most tightly packed players.
///
/// Every combination of `n` is tried and the one with the smallest bounding
/// sphere — largest pairwise distance, halved — wins. Brute force, but `n` is
/// two in every rotation entry that exists, and the result is deterministic,
/// which matters more here than it did in Bevy: a tick is a transaction, and a
/// transaction that picks a different answer on a replay is a bug. (The Bevy
/// version iterated a `HashMap` and said so in its own doc comment.)
///
/// `None` when fewer than `n` players are alive.
fn densest_cluster(players: &[PlayerRef], n: usize) -> Option<Vec3> {
    if n == 0 || players.len() < n {
        return None;
    }
    let considered = players.len().min(CLUSTER_CANDIDATE_LIMIT);
    let indices: Vec<usize> = (0..considered).collect();
    let mut best: Option<(f32, Vec3)> = None;

    for combo in combinations(&indices, n) {
        let spread = max_pairwise_distance(&combo, players);
        if spread.is_nan() {
            continue;
        }
        let centroid = combo
            .iter()
            .map(|&i| players[i].position)
            .fold(Vec3::ZERO, |sum, position| sum + position)
            / n as f32;
        if best.is_none_or(|(best_spread, _)| spread < best_spread) {
            best = Some((spread, centroid));
        }
    }
    best.map(|(_, centroid)| centroid)
}

/// Every length-`k` combination of `indices`.
fn combinations(indices: &[usize], k: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    let mut current: Vec<usize> = Vec::with_capacity(k);
    combine_recursive(indices, 0, k, &mut current, &mut out);
    out
}

fn combine_recursive(
    indices: &[usize],
    start: usize,
    k: usize,
    current: &mut Vec<usize>,
    out: &mut Vec<Vec<usize>>,
) {
    if current.len() == k {
        out.push(current.clone());
        return;
    }
    for i in start..indices.len() {
        current.push(indices[i]);
        combine_recursive(indices, i + 1, k, current, out);
        current.pop();
    }
}

fn max_pairwise_distance(combo: &[usize], players: &[PlayerRef]) -> f32 {
    let mut max_squared = 0.0_f32;
    for a in 0..combo.len() {
        for b in (a + 1)..combo.len() {
            let distance = players[combo[a]]
                .position
                .distance_squared(players[combo[b]].position);
            if distance > max_squared {
                max_squared = distance;
            }
        }
    }
    max_squared.sqrt()
}

// ---------------------------------------------------------------------------
// Spatial queries
// ---------------------------------------------------------------------------

/// The nearest living player within `radius` of `origin`.
fn nearest_living_player(ctx: &ReducerContext, origin: Vec3, radius: f32) -> Option<PlayerRef> {
    let candidates = living_players_near(ctx, origin, radius);
    let nearest = nearest_target(&candidates, origin)?;
    Some(PlayerRef {
        entity: nearest.entity,
        position: nearest.position,
    })
}

/// Every living player within `radius` of `center`, found through the grid.
///
/// The index is `(cell_x, cell_z)`, and a multi-column btree only accepts exact
/// values for the columns before the last — so the scan is one `filter` per
/// `cell_x` column with a range over `cell_z`, rather than one call for the
/// rectangle. The cells are a conservative superset of the circle (they are
/// squares, and they ignore `y`), so each candidate still gets a real distance
/// test.
///
/// `state` is what aliveness is read from, not `entity_stats`, because
/// `sim::combat::reap_the_dead` settled the two one step ago — which saves a
/// point lookup per candidate.
fn living_players_near(ctx: &ReducerContext, center: Vec3, radius: f32) -> Vec<PlayerRef> {
    let mut found = Vec::new();
    if radius.is_nan() || radius <= 0.0 {
        return found;
    }
    let radius = radius.min(MAX_SPATIAL_QUERY_RADIUS);

    let (min_x, min_z) = grid_cell(Vec3Row::from(center - Vec3::splat(radius)));
    let (max_x, max_z) = grid_cell(Vec3Row::from(center + Vec3::splat(radius)));
    let radius_squared = radius * radius;
    let online = targets::online_character_ids(ctx);

    for cell_x in min_x..=max_x {
        for entity in ctx.db.game_entity().cell().filter((cell_x, min_z..=max_z)) {
            let online_flag = entity.owner_character_id.map(|id| online.contains(&id));
            if !targets::is_online_living_player(entity.kind, entity.state, online_flag) {
                continue;
            }
            let position = Vec3::from(entity.position);
            if position.distance_squared(center) > radius_squared {
                continue;
            }
            found.push(PlayerRef {
                entity: entity.entity_id,
                position,
            });
        }
    }
    found
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Drops a chosen destination when the mob is in no position to walk to it.
///
/// Two reasons it may not:
///
/// - It is rooted or stunned. `crowd_control::step` already cleared whatever it
///   had; this is what stops the AI handing it a fresh one every tick.
/// - It is mid-cast. `sim::spells::advance_casts` interrupts a cast-time spell
///   whose caster moved, so a dragon that kept walking into melee would cancel
///   its own searing breath on the tick after it started it. Bevy had the same
///   two systems and the same collision; there the boss simply lost the cast.
fn gate_movement(
    ctx: &ReducerContext,
    entity_id: u64,
    move_target: Option<Vec3Row>,
) -> Option<Vec3Row> {
    move_target?;
    if crowd_control::is_movement_blocked(ctx, entity_id) {
        return None;
    }
    if ctx.db.cast_state().entity_id().find(entity_id).is_some() {
        return None;
    }
    move_target
}

/// Writes a mob's facing and destination, and hands back the current row.
///
/// Skips the write when nothing changed: a mob standing at its spawn with
/// nobody around should cost the database nothing per tick.
fn write_pose(
    ctx: &ReducerContext,
    entity: GameEntity,
    look: Vec3Row,
    move_target: Option<Vec3Row>,
) -> GameEntity {
    let changed = look != entity.look || move_target != entity.move_target;
    let entity = GameEntity {
        look,
        move_target,
        ..entity
    };
    if changed {
        ctx.db.game_entity().entity_id().update(entity)
    } else {
        entity
    }
}

/// `current_health / max_health`, clamped, and zero when there is no health to
/// speak of.
fn health_fraction(ctx: &ReducerContext, entity_id: u64) -> f32 {
    let Some(stats) = ctx.db.entity_stats().entity_id().find(entity_id) else {
        return 0.0;
    };
    if stats.stats.max_health <= 0.0 {
        return 0.0;
    }
    (stats.stats.current_health / stats.stats.max_health).clamp(0.0, 1.0)
}

/// Whether `entity_id` is free to begin a cast: not already casting, not
/// silenced, not stunned.
///
/// The "already casting" half is what Bevy expressed as `Without<CastProgress>`
/// on the rotation query. The CC half goes through `crowd_control` rather than
/// `spells::casting_blocked`, which is the same predicate spelled twice — see
/// the port report.
fn can_start_cast(ctx: &ReducerContext, entity_id: u64) -> bool {
    !crowd_control::is_casting_blocked(ctx, entity_id)
        && ctx.db.cast_state().entity_id().find(entity_id).is_none()
}

/// Starts a spell on an AI actor's behalf.
///
/// The Bevy systems wrote a `SpellCastRequest` and let `process_cast_requests`
/// validate it. There is no message queue here and, deliberately, no reducer:
/// `reducers::spells::cast_spell` authenticates a *caller* and enforces a
/// player's three-slot hotbar, neither of which a mob has. So this composes the
/// same pipeline from `sim::spells`' public parts — the shared registry, the
/// shared `fire_spell`, the shared cooldown — and the only thing it does not
/// borrow is the hotbar check, replaced upstream by the boss spellbook and by
/// the enemy only ever asking for `attack`.
///
/// The caller must have checked [`can_start_cast`]: `cast_state` is keyed by
/// caster, so opening a second cast on one would be an insert conflict.
fn request_cast(
    ctx: &ReducerContext,
    caster: &GameEntity,
    spell_id: &SpellId,
    target_entity: Option<u64>,
    target_position: Option<Vec3>,
) {
    let Some(spell) = spells::spells().get(spell_id) else {
        // A rotation entry with no implementation registered. Worth saying out
        // loud: the fight silently loses an ability otherwise.
        log::warn!("ai: no spell registered for {:?}", spell_id.as_str());
        return;
    };
    let config = spell.config();
    let kind = spell.cast_kind();

    match kind {
        CastKind::Instant => {
            if let Some(cooldown_seconds) =
                spells::fire_spell(ctx, caster, spell.as_ref(), target_position, target_entity)
            {
                spells::start_cooldown(ctx, caster.entity_id, spell_id.as_str(), cooldown_seconds);
            }
        }
        CastKind::CastTime | CastKind::Channeling => {
            let channeling = matches!(kind, CastKind::Channeling);
            let required_seconds = if channeling {
                // A player ends a channel by releasing the key. An AI has no
                // key, so a channel with no declared duration would run until
                // something else interrupted it. Skipping the ability costs the
                // boss one entry in its rotation; casting it would cost every
                // other entry, permanently.
                let Some(duration) = config.channel_duration_seconds else {
                    log::warn!(
                        "ai: {:?} channels with no duration; skipping",
                        spell_id.as_str()
                    );
                    return;
                };
                duration
            } else {
                config.cast_time_seconds
            };
            let tick_interval_seconds = spell.channel_tick_interval_seconds();

            // Both details mirror `reducers::spells::cast_spell`: a channel
            // starts armed so its first tick lands immediately, and its
            // cooldown starts now rather than on completion, so a long channel
            // cannot also delay the next cast.
            let channel_tick_accumulator = if channeling {
                tick_interval_seconds
            } else {
                0.0
            };
            if channeling {
                spells::start_cooldown(
                    ctx,
                    caster.entity_id,
                    spell_id.as_str(),
                    config.cooldown_seconds,
                );
            }

            ctx.db.cast_state().insert(CastState {
                entity_id: caster.entity_id,
                spell_id: spell_id.as_str().to_string(),
                kind: spells::cast_kind_row(kind),
                source: crate::tables::CastSourceRow::Spell,
                elapsed_seconds: 0.0,
                required_seconds,
                start_position: caster.position,
                target_position: target_position.map(Vec3Row::from),
                target_entity,
                channel_tick_accumulator,
                tick_interval_seconds,
                // Boss AI always uses legacy spell path; read from SpellConfig.
                channel_movement_interrupts: matches!(
                    kind,
                    CastKind::Channeling if config.channel_movement == SpellChannelMovementPolicy::InterruptOnMove
                ),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Phase mapping
// ---------------------------------------------------------------------------

/// `BossPhaseRow` -> `BossPhase`.
///
/// The two enums are the same machine under different names: the schema calls
/// the phases by number, the domain calls them by what the dragon is doing.
/// `BossPhase::Dead` has no row spelling — a defeated boss is one whose
/// `game_entity.state` is `Dead` — so the mapping is total in this direction
/// and lossy in the other.
fn phase_from_row(phase: BossPhaseRow) -> BossPhase {
    match phase {
        BossPhaseRow::Idle => BossPhase::Dormant,
        BossPhaseRow::PhaseOne => BossPhase::Ground,
        BossPhaseRow::PhaseTwo => BossPhase::Aerial,
        BossPhaseRow::Enraged => BossPhase::Berserk,
    }
}

/// `BossPhase` -> `BossPhaseRow`.
///
/// `Dead` maps to `Enraged`, the last phase a boss can die in; it is
/// unreachable in practice, because a dead boss never reaches the phase
/// machine.
fn phase_to_row(phase: BossPhase) -> BossPhaseRow {
    match phase {
        BossPhase::Dormant => BossPhaseRow::Idle,
        BossPhase::Ground => BossPhaseRow::PhaseOne,
        BossPhase::Aerial => BossPhaseRow::PhaseTwo,
        BossPhase::Berserk | BossPhase::Dead => BossPhaseRow::Enraged,
    }
}
