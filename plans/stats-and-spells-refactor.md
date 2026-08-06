# Plan: Stats and Spells Refactor

**Status**: planned  
**Scope**: remove the current embedded stats/combat implementation, introduce a dedicated Stats plugin, introduce a dynamic trait-based Spells plugin under `src/plugins/spells/`, keep concrete spell content under `src/spells/`, add room for a future shared proc-macro crate inside a workspace, and persist player stats in PostgreSQL.

## Goal

Replace the current ad-hoc combat/stat system with two explicit gameplay capabilities:

1. **Stats Plugin**
   - Owns all runtime stat components such as movement, combat, and vital stats.
   - Exposes an aggregate `StatsBundleData` for spawn/config/persistence, while keeping runtime ECS components split.
   - Provides safe APIs/systems for damage, healing, stat modifiers, death state updates, and stat initialization.
   - Supports player stat persistence through the existing SeaORM/PostgreSQL layer.

2. **Spells Plugin**
   - The generic ECS/plugin infrastructure lives under `src/plugins/spells/`.
   - The concrete spell catalog lives under `src/spells/`, with one submodule per spell (`attack`, later `fireball`, `frostball`, `lightball`, ...).
   - Provides a good developer experience for adding new spells with a dynamic trait-based registry.
   - Starts with one spell: `Attack`, equivalent in behavior to the current enemy area attack.
   - Moves all attack-specific logic out of `plugins/entity/enemy/systems.rs` and out of `EnemyAttack`.

The final result should make adding a new spell feel like:

```rust
pub struct Fireball;

impl Spell for Fireball {
    fn id(&self) -> SpellId { SpellId::new("fireball") }
    fn display_name(&self) -> &'static str { "Fireball" }
    fn config(&self) -> SpellConfig { /* cooldown, range, etc. */ }
    fn can_cast(&self, ctx: &SpellCastContext) -> SpellCastResult { /* optional */ }
    fn cast(&self, ctx: &mut SpellCastContext) -> SpellCastResult { /* gameplay effect */ }
}
```

and registering it should be a one-line operation in the Spells plugin.

---

## Current System Research

### Current shared entity components

File: `src/plugins/entity/components.rs`

Currently this file owns both entity identity/state and gameplay stats:

- `GameEntity`
- `EntityState`
- `Health`
- `Stats`
- `PlayerName`

`Stats` currently contains mixed concerns:

```rust
pub struct Stats {
    pub speed: f32,
    pub damage: f32,
    pub max_health: f32,
    pub max_mana: f32,
    pub mana_regeneration: f32,
    pub armor: f32,
}
```

Problems:

- Movement, combat damage, mana, health metadata, and armor all live in one generic component.
- `Health` is defined next to generic entity identity, not inside a dedicated stats/attributes feature.
- `Stats::damage_reduction()` is combat logic embedded in a data component.
- The current player and enemy default stats are static code defaults, not loaded from DB for players.

### Current attack implementation

Files:

- `src/plugins/entity/enemy/components.rs`
- `src/plugins/entity/enemy/systems.rs`
- `src/plugins/entity/enemy/mod.rs`
- `src/plugins/entity/enemy/debug.rs`
- `src/plugins/entity/enemy/spawn.rs`

Current attack data:

```rust
pub struct EnemyAttack {
    pub radius: f32,
    pub cooldown_seconds: f32,
}
```

Current runtime cooldown:

```rust
pub struct EnemyAttackCooldown(pub Timer);
```

Current attack system:

```rust
pub fn enemy_area_attack(
    time: Res<Time<Fixed>>,
    mut enemies: Query<(&Position, &Stats, &EnemyAttack, &mut EnemyAttackCooldown), With<Enemy>>,
    mut players: Query<(&Position, &mut Health, &Stats), (With<Player>, Without<Enemy>)>,
)
```

Behavior:

1. Each enemy has an `EnemyAttack` component.
2. `initialize_attack_cooldowns` inserts a repeating timer.
3. Every fixed tick, `enemy_area_attack` advances the timer.
4. When the timer finishes, every player inside `attack.radius` takes damage.
5. Damage is `enemy_stats.damage` reduced by target armor.
6. `health.current` is reduced and clamped to `0.0`.

Current equivalent default values:

- Enemy attack radius: `3.0`
- Enemy attack cooldown: `1.0s`
- Enemy damage: `18.0`
- Enemy armor: `10.0`
- Player armor: `25.0`

### Current plugin registration

File: `src/plugins/entity/enemy/mod.rs`

```rust
app.add_systems(
    FixedUpdate,
    (
        systems::enemy_chase,
        systems::initialize_attack_cooldowns,
        systems::enemy_area_attack,
    )
        .chain()
        .run_if(crate::network::mode::has_server),
);
```

The attack is correctly server-authoritative today, but it is hard-coded to Enemy and cannot scale to multiple spells.

### Current network protocol registration

File: `src/network/protocol.rs`

Currently replicated/predicted gameplay components include:

```rust
app.component::<Stats>().replicate().predict();
app.component::<EnemyAttack>().replicate();
app.component::<EntityState>().replicate().predict();
app.component::<GameEntity>().replicate();
app.component::<Health>().replicate().predict();
```

Problems:

- `Stats` and `Health` are imported from `plugins::entity::components`.
- `EnemyAttack` is replicated only for debug visualization, not because clients need it for authoritative gameplay.
- Any new stats module must update protocol registration.

### Current movement dependency on Stats

File: `src/plugins/player_movement.rs`

Movement reads `Stats.speed`:

```rust
move_towards_target(position, &input.0, stats.speed, state);
```

The refactor must preserve predicted movement by providing a replacement movement-speed stat component or stat accessor.

### Current UI dependency on Stats and Health

Files:

- `src/ui/player_stats/systems.rs`
- `src/ui/entity_bar/systems.rs`

Current player stats UI reads:

- `Stats.max_health`
- `Stats.max_mana`
- `Stats.mana_regeneration`
- `Stats.armor`
- `Stats::damage_reduction()`

Current floating entity bar reads:

- `Health.current`
- `Health.max`

The refactor must update these UI systems so the current HP comes from `Health.current`, while max HP comes from the new Stats API (`Stats.max_health`).

### Current persistence system

Files:

- `src/persistence/entity/player.rs`
- `src/persistence/repository/player.rs`
- `src/persistence/migration.rs`
- `src/network/server.rs`

Current `players` table stores:

- `id`
- `normalized_name`
- `display_name`
- `pos_x`
- `pos_y`
- `pos_z`

Current join flow:

1. Client sends `JoinRequest`.
2. Server validates player name.
3. Server calls `PlayerRepository::find_or_create` asynchronously.
4. Result returns `PlayerRecord`.
5. `finish_pending_joins` spawns the player with:

```rust
Player::health()
Player::stats()
```

Current disconnect flow:

1. Server finds player by `PlayerId` / `DbPlayerId`.
2. Server saves position only through `save_position`.
3. Server despawns the player.

Problems:

- Stats are not loaded from DB.
- Stats are not saved to DB.
- `PlayerRecord` cannot represent a full gameplay snapshot.

---

## Target Architecture

### New top-level modules

```text
Cargo.toml                         # workspace root
crates/
└── gameplay_macros/               # future shared proc-macro crate, not required in slice 1
    └── Cargo.toml
src/
├── stats/
│   ├── mod.rs
│   ├── components.rs
│   ├── plugin.rs
│   ├── systems.rs
│   ├── formulas.rs
│   ├── defaults.rs
│   └── events.rs
├── plugins/
│   ├── spells/
│   │   ├── mod.rs
│   │   ├── plugin.rs
│   │   ├── registry.rs
│   │   ├── components.rs
│   │   ├── events.rs
│   │   ├── context.rs
│   │   └── systems.rs
│   └── enemies/                  # future generic enemy ECS/plugin infrastructure
│       ├── mod.rs
│       ├── plugin.rs
│       ├── components.rs
│       └── systems.rs
├── spells/                       # concrete spell catalog/content
│   ├── mod.rs
│   ├── attack/
│   │   ├── mod.rs
│   │   ├── definition.rs
│   │   └── tests.rs              # optional; inline tests are also fine
│   ├── fireball/                 # future
│   ├── frostball/                # future
│   └── lightball/                # future
└── enemies/                      # future concrete enemy catalog/content
    ├── mod.rs
    ├── normal_mob/
    ├── boss_mob/
    └── lord_boss_mob/
```

Important ownership rule:

- `src/plugins/spells/` owns the reusable spell framework: plugin registration, trait, registry, ECS components, events, context, cooldown systems, and cast dispatch.
- `src/spells/` owns concrete spell content: `Attack`, later `Fireball`, `Frostball`, `Lightball`, etc.
- `src/plugins/enemies/` should eventually own generic enemy framework/AI systems.
- `src/enemies/` should eventually own concrete enemy definitions: `NormalMob`, `BossMob`, `LordBossMob`, etc.

This keeps framework code separate from content code and makes adding new gameplay content a directory-level operation instead of editing a monolithic module.

The workspace/proc-macro crate is a future-facing structural choice. The first refactor should not depend on custom spell or entity proc macros.

### Stats module responsibilities

The Stats module should own:

- Split runtime ECS stat components.
- An aggregate `StatsBundleData` used at spawn/config/persistence boundaries.
- Derived formulas such as armor reduction.
- Damage/heal application events.
- Generic stat modifier application and expiration.
- Death state synchronization if needed.
- Default stat profiles for Player and Enemy.
- Serialization-compatible stat snapshots for persistence/networking.

Recommended runtime ECS components:

```rust
#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq)]
pub struct MovementStats {
    pub speed: f32,
}

#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq)]
pub struct CombatStats {
    pub attack_power: f32,
    pub armor: f32,
}

#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq)]
pub struct VitalStats {
    pub current_health: f32,
    pub max_health: f32,
    pub max_mana: f32,
    pub mana_regeneration: f32,
}
```

Recommended aggregate data type:

```rust
pub struct StatsBundleData {
    pub movement: MovementStats,
    pub combat: CombatStats,
    pub vital: VitalStats,
}
```

`StatsBundleData` is not meant to replace the ECS split. It is a convenient aggregate for:

- entity defaults
- spawn helpers
- persistence serialization/deserialization
- future content definitions

`current_health` lives inside `VitalStats`, not in a separate `Health` component.

### Stats plugin responsibilities

`StatsPlugin` should register:

- Reflection for `MovementStats`, `CombatStats`, and `VitalStats`.
- Server-side systems that process damage/heal/death events.
- Server-side systems that apply and expire stat modifiers.
- Tests for stat formulas.

Example:

```rust
pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MovementStats>();
        app.register_type::<CombatStats>();
        app.register_type::<VitalStats>();
        app.add_event::<DamageEvent>();
        app.add_event::<HealEvent>();
        app.add_event::<ApplyStatModifierEvent>();
        app.add_systems(FixedUpdate, apply_damage.run_if(mode::has_server));
        app.add_systems(FixedUpdate, apply_healing.run_if(mode::has_server));
        app.add_systems(FixedUpdate, apply_stat_modifiers.run_if(mode::has_server));
        app.add_systems(FixedUpdate, tick_stat_modifiers.run_if(mode::has_server));
    }
}
```

Network replication can remain centralized in `network::protocol::ProtocolPlugin`, but it must import the new stat component types from `crate::stats::components`.

### Stat modifiers architecture

Do not mutate base stat values directly for temporary effects. Temporary buffs/debuffs must go through the modifier pipeline so expiration, stacking, dispelling, and recalculation stay predictable.

Data model:

```rust
pub enum StatField {
    Speed,
    Armor,
    AttackPower,
    MaxHealth,
    ManaRegeneration,
}

pub enum ModifierOp {
    Add,
    Multiply,
    Override,
}

pub enum ModifierKind {
    Buff,
    Debuff,
}

pub struct ApplyStatModifierEvent {
    pub target: Entity,
    pub source: Option<Entity>,
    pub field: StatField,
    pub operation: ModifierOp,
    pub value: f32,
    pub duration_seconds: Option<f32>,
    pub kind: ModifierKind,
}
```

Runtime component:

```rust
#[derive(Component, Default)]
pub struct ActiveStatModifiers {
    pub modifiers: Vec<StatModifierInstance>,
}

pub struct StatModifierInstance {
    pub id: ModifierId,
    pub source: Option<Entity>,
    pub field: StatField,
    pub operation: ModifierOp,
    pub value: f32,
    pub remaining_seconds: Option<f32>,
    pub kind: ModifierKind,
}
```

Flow:

```text
Spell / ability / gear
        |
        v
ApplyStatModifierEvent
        |
        v
StatsPlugin: apply_stat_modifiers adds a StatModifierInstance
        |
        v
StatsPlugin: tick_stat_modifiers decrements/expiry removes
        |
        v
Gameplay systems read effective values (base +/- modifiers)
```

Rules:

- `VitalStats.current_health` is never used as a `StatField` for modifiers. Use `DamageEvent` / `HealEvent` for current HP changes.
- `VitalStats.max_health` can be a modifier target. When it changes, decide explicitly what happens to `current_health` (clamp, scale, leave unchanged).
- Persisted stats are the base values. `ActiveStatModifiers` is transient and is not persisted unless gameplay explicitly requires it.

Future enhancement (not required now):

- a proc macro inside `crates/gameplay_macros/` could derive `StatField` automatically from the stat struct fields, removing the need to maintain the enum by hand.

### Spells framework and catalog responsibilities

The generic Spells framework under `src/plugins/spells/` should own:

- Spell identity.
- Spell trait.
- Spell registry.
- Spell runtime state such as known spells and cooldowns.
- Spell cast requests/events.
- Server-authoritative spell execution.
- Optional client debug visuals for spell areas.

The concrete spell catalog under `src/spells/` should own:

- One directory per spell.
- The `Attack` spell implementation.
- Future `Fireball`, `Frostball`, `Lightball`, etc.
- Spell-specific constants, tests, and effect implementation details.

Recommended data model:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpellId(&'static str);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpellConfig {
    pub cooldown_seconds: f32,
    pub cast_range: f32,
    pub area_radius: f32,
}
```

Runtime components:

```rust
#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Spellbook {
    pub spells: Vec<SpellId>,
}

#[derive(Component, Default)]
pub struct SpellCooldowns {
    pub timers: HashMap<SpellId, Timer>,
}
```

For Bevy/Lightyear compatibility, if `HashMap<SpellId, Timer>` is awkward for replication, keep cooldowns server-only and replicate only debug-visible config if needed.

---

## Dynamic Trait-Based Spell Design

### Spell trait

The trait should be object-safe so spells can be stored as `Box<dyn Spell>` or `Arc<dyn Spell>` in a registry resource.

Recommended shape:

```rust
pub trait Spell: Send + Sync + 'static {
    fn id(&self) -> SpellId;
    fn name(&self) -> &'static str;
    fn config(&self) -> SpellConfig;

    fn can_cast(&self, ctx: &SpellCastContext) -> Result<(), SpellCastError> {
        ctx.default_can_cast(self.config())
    }

    fn cast(&self, ctx: &mut SpellCastContext) -> Result<(), SpellCastError>;
}
```

The important DX constraint: spell authors should not need to write Bevy systems for every spell. They should implement a trait and register the spell.

### Spell registry

```rust
#[derive(Resource, Default)]
pub struct SpellRegistry {
    spells: HashMap<SpellId, Arc<dyn Spell>>,
}

impl SpellRegistry {
    pub fn register<S: Spell>(&mut self, spell: S) -> &mut Self;
    pub fn get(&self, id: SpellId) -> Option<Arc<dyn Spell>>;
}
```

Registration example:

```rust
impl Plugin for SpellsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpellRegistry>();
        app.add_systems(Startup, register_builtin_spells);
        app.add_systems(FixedUpdate, cast_requested_spells.run_if(mode::has_server));
    }
}

fn register_builtin_spells(mut registry: ResMut<SpellRegistry>) {
    registry.register(AttackSpell);
}
```

### Spell cast context

`SpellCastContext` should give spells ergonomic access to the world data they need without each spell becoming a full Bevy system.

There are two viable designs:

#### Option A — Command/event based context, preferred for safety

The spell computes effects and emits events:

```rust
pub struct SpellCastContext<'a> {
    pub caster: Entity,
    pub target: SpellTarget,
    pub caster_position: Vec3,
    pub caster_combat: CombatStats,
    pub world_query: SpellWorldQuery<'a>,
    pub damage_events: EventWriter<'a, DamageEvent>,
    pub heal_events: EventWriter<'a, HealEvent>,
    pub stat_modifier_events: EventWriter<'a, ApplyStatModifierEvent>,
}
```

Pros:

- Easy to test.
- Keeps mutation centralized in Stats systems.
- Avoids handing full mutable `World` access to arbitrary spell code.

Cons:

- The context must expose enough query helpers for common spell patterns.

#### Option B — Exclusive world context

```rust
fn cast(&self, ctx: &mut SpellCastContext, world: &mut World) -> Result<(), SpellCastError>;
```

Pros:

- Maximum flexibility.

Cons:

- Harder to schedule safely.
- Easier to create borrow conflicts.
- Worse DX for simple spells.

Use **Option A** first.

### Cast flow

Recommended server-authoritative flow:

```text
Caster AI / client input
        |
        v
SpellCastRequest event
        |
        v
cast_requested_spells system
        |
        v
SpellRegistry lookup
        |
        v
Spell::can_cast
        |
        v
Spell::cast
        |
        v
DamageEvent / HealEvent / ApplyStatModifierEvent / SpawnProjectile / etc.
        |
        v
StatsPlugin systems apply effects
        |
        v
Health/Stats replicated to clients
```

### Attack spell

`Attack` should replicate the current behavior:

- Area-of-effect spell.
- Cast by Enemy in the current implementation.
- Cooldown: `1.0s`.
- Radius: `3.0`.
- Damage source: caster `CombatStats.attack_power`.
- Targets: all Players within radius.
- Damage reduction: target armor formula.
- `VitalStats.current_health` clamped to zero.

Implementation should move current logic from:

- `enemy_area_attack`
- `is_in_attack_radius`
- `damage_after_armor`
- `EnemyAttack`
- `EnemyAttackCooldown`

into:

- `src/spells/attack/definition.rs`
- `src/plugins/spells/systems.rs`
- `src/stats/formulas.rs`
- `src/stats/systems.rs`

---

## Persistence Target

### Recommended DB design

Use a separate `player_stats` table instead of adding many stat columns to `players`.

Reasoning:

- `players` already stores identity and position.
- Stats will grow over time.
- A separate table makes stat migrations easier and avoids turning `players` into a wide mixed-concern table.

Target schema:

```sql
CREATE TABLE player_stats (
    player_id UUID PRIMARY KEY REFERENCES players(id) ON DELETE CASCADE,
    current_health REAL NOT NULL,
    max_health REAL NOT NULL,
    max_mana REAL NOT NULL,
    mana_regeneration REAL NOT NULL,
    armor REAL NOT NULL,
    movement_speed REAL NOT NULL,
    attack_power REAL NOT NULL
);
```

This schema maps naturally to `StatsBundleData`:

- `movement_speed` -> `MovementStats.speed`
- `attack_power` / `armor` -> `CombatStats`
- `current_health` / `max_health` / `max_mana` / `mana_regeneration` -> `VitalStats`

Optional future fields:

- `current_mana`
- `level`
- `experience`
- `strength`
- `intelligence`
- `agility`
- `updated_at`

### SeaORM changes

Add:

```text
src/persistence/entity/player_stats.rs
```

Update:

```text
src/persistence/entity/mod.rs
src/persistence/repository/player.rs
src/persistence/migration.rs
```

Repository should return a complete loaded player snapshot:

```rust
pub struct PersistedPlayerSnapshot {
    pub player: PlayerRecord,
    pub stats: StatsBundleData,
}
```

Possible repository API:

```rust
pub async fn find_or_create_snapshot(
    &self,
    normalized_name: &str,
    display_name: &str,
) -> PersistenceResult<PersistedPlayerSnapshot>;

pub async fn save_snapshot(
    &self,
    id: Uuid,
    position: Vec3,
    stats: StatsBundleData,
) -> PersistenceResult<()>;
```

For better incremental adoption, keep `save_position` temporarily and add `save_stats`, then merge into `save_snapshot` once call sites are stable.

### Migration behavior

Add a new migration after `m20260805_000001_create_players`:

```rust
m20260805_000002_create_player_stats
```

Migration should:

1. Create `player_stats` table if it does not exist.
2. Insert default stat rows for existing players.
3. Use Player default stats for backfill.

Because SeaORM migrations can execute raw SQL, backfill can be:

```sql
INSERT INTO player_stats (...)
SELECT id, 100.0, 100.0, 100.0, 5.0, 25.0, 0.15, 10.0
FROM players
ON CONFLICT (player_id) DO NOTHING;
```

### Join flow changes

Current:

```rust
repository.find_or_create(...).await -> PlayerRecord
```

Target:

```rust
repository.find_or_create_snapshot(...).await -> PersistedPlayerSnapshot
```

Then `finish_pending_joins` spawns the player with DB-loaded stats:

```rust
GameEntityBundle::new(
    position,
    EntityColor(color),
    snapshot.stats.health(),
    snapshot.stats.stats(),
    NetworkTarget::All,
)
```

### Disconnect flow changes

Current disconnect saves only position.

Target disconnect should save:

- Position.
- Current and max health from `VitalStats`.
- Movement values from `MovementStats`.
- Combat values from `CombatStats`.
- Any other persisted resources.

The query should change from:

```rust
players: Query<(Entity, &PlayerId, &DbPlayerId, &Position), With<Player>>
```

to:

```rust
players: Query<(
    Entity,
    &PlayerId,
    &DbPlayerId,
    &Position,
    &MovementStats,
    &CombatStats,
    &VitalStats,
), With<Player>>
```

Then call a repository method that persists all relevant data.

---

## Detailed Implementation Slices

### Slice 1 — Create the Stats module without changing behavior

Create files:

```text
src/stats/mod.rs
src/stats/components.rs
src/stats/formulas.rs
src/stats/defaults.rs
src/stats/systems.rs
src/stats/plugin.rs
```

Move/introduce into `src/stats/components.rs`:

- `MovementStats`
- `CombatStats`
- `VitalStats`
- `StatsBundleData`
- modifier runtime types such as `ActiveStatModifiers`

Move formulas into `src/stats/formulas.rs`:

- `armor_damage_reduction`
- `damage_after_armor`
- future modifier aggregation helpers

Recommended API:

```rust
impl CombatStats {
    pub fn armor_damage_reduction(&self) -> f32;
}

impl StatsBundleData {
    pub fn into_components(self) -> (MovementStats, CombatStats, VitalStats);
}

pub fn damage_after_armor(raw_damage: f32, target_stats: &CombatStats) -> f32;
```

Update imports in:

- `src/network/protocol.rs`
- `src/plugins/entity/definition.rs`
- `src/plugins/entity/spawn.rs`
- `src/plugins/entity/player/spawn.rs`
- `src/plugins/entity/enemy/spawn.rs`
- `src/plugins/entity/enemy/systems.rs`
- `src/plugins/player_movement.rs`
- `src/ui/player_stats/systems.rs`
- `src/ui/entity_bar/systems.rs`
- `src/plugins/entity/systems.rs`

Do not keep the old `Health` / `Stats` split if it fights the target model. It is better to migrate directly to `MovementStats`, `CombatStats`, `VitalStats`, and `StatsBundleData` even if the diff is larger.

Register `StatsPlugin` in `src/main.rs`:

```rust
mod stats;

app.add_plugins(stats::StatsPlugin);
```

Placement recommendation:

- Add before `EntityPlugin` and `PlayerMovementPlugin`.
- Keep `ProtocolPlugin` responsible for Lightyear component replication.

Validation:

```bash
cargo fmt --check
cargo test stats
cargo check
```

### Slice 2 — Remove entity ownership of stats

After the code compiles with moved imports:

1. Delete old stat definitions from `plugins/entity/components.rs`.
2. Update entity module comments to stop claiming that entity owns health/stats.
3. Update `EntityDefinition` imports to use `crate::stats::components`.
4. Update `GameEntityBundle` and related spawn code to use `MovementStats`, `CombatStats`, and `VitalStats`, ideally via `StatsBundleData::into_components()`.

Validation:

```bash
cargo test
cargo check
```

### Slice 3 — Add Stats persistence

Create:

```text
src/persistence/entity/player_stats.rs
```

Update:

```text
src/persistence/entity/mod.rs
src/persistence/migration.rs
src/persistence/repository/player.rs
src/persistence/mod.rs
src/network/server.rs
```

Implementation steps:

1. Add SeaORM entity for `player_stats`.
2. Add migration `m20260805_000002_create_player_stats`.
3. Add conversion between DB rows and runtime stats:

```rust
impl StatsBundleData {
    pub fn from_components(
        movement: &MovementStats,
        combat: &CombatStats,
        vital: &VitalStats,
    ) -> Self;

    pub fn into_components(self) -> (MovementStats, CombatStats, VitalStats);
}
```

4. Change join result type from `PlayerRecord` to `PersistedPlayerSnapshot`.
5. Spawn player using persisted health/stats.
6. Save health/stats on disconnect.

Important: keep DB work asynchronous through the existing `PersistenceRuntime`; do not `.await` inside Bevy systems.

Validation:

```bash
cargo fmt --check
cargo test persistence
cargo check
```

Manual validation:

1. Start server with a test database.
2. Join with a new player.
3. Verify both `players` and `player_stats` rows exist.
4. Change health/stats through gameplay.
5. Disconnect.
6. Rejoin.
7. Verify loaded stats match saved stats.

### Slice 4 — Create the Spells framework and catalog split

Create generic framework files:

```text
src/plugins/spells/mod.rs
src/plugins/spells/plugin.rs
src/plugins/spells/registry.rs
src/plugins/spells/components.rs
src/plugins/spells/events.rs
src/plugins/spells/context.rs
src/plugins/spells/systems.rs
src/stats/events.rs
```

Create concrete spell catalog files:

```text
src/spells/mod.rs
src/spells/attack/mod.rs
src/spells/attack/definition.rs
```

Add:

```rust
mod spells;
```

in `src/main.rs` for the concrete spell catalog.

Update:

```rust
pub mod spells;
```

in `src/plugins/mod.rs` for the plugin/framework module.

Register:

```rust
app.add_plugins(plugins::spells::SpellsPlugin);
```

Placement recommendation:

- After `StatsPlugin`.
- Before `EntityPlugin` or after `EntityPlugin` depending on whether spell systems query marker components from `entity`.
- If spells reference `Player`/`Enemy`, registering after `EntityPlugin` is acceptable, but module imports should avoid circular plugin ownership.

Initial SpellsPlugin:

```rust
pub struct SpellsPlugin;

impl Plugin for SpellsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpellRegistry>();
        app.add_event::<SpellCastRequest>();
        app.add_systems(Startup, register_builtin_spells);
        app.add_systems(FixedUpdate, cast_requested_spells.run_if(mode::has_server));
    }
}
```

Validation:

```bash
cargo fmt --check
cargo test spells
cargo check
```

### Slice 5 — Recreate current Attack as a spell

Implement `src/spells/attack/definition.rs` with current behavior:

```rust
pub struct AttackSpell;
```

Defaults:

```rust
cooldown_seconds: 1.0
area_radius: 3.0
cast_range: 0.0 // optional if centered on caster
```

Attack behavior:

1. Use caster position as center.
2. Find players within radius.
3. Compute damage from `caster_combat.attack_power`.
4. Apply armor reduction using combat formulas.
5. Emit `DamageEvent` for each target.

Move tests from `enemy/systems.rs` into:

- `stats/formulas.rs` tests for armor and damage math.
- `spells/attack/definition.rs` or `spells/attack/tests.rs` tests for radius inclusion/exclusion.

Validation:

```bash
cargo test attack
cargo test stats
cargo check
```

### Slice 6 — Replace EnemyAttack with Spellbook + cooldowns

Remove from `src/plugins/entity/enemy/components.rs`:

- `EnemyAttack`

Remove from `src/plugins/entity/enemy/systems.rs`:

- `EnemyAttackCooldown`
- `initialize_attack_cooldowns`
- `enemy_area_attack`
- attack radius helper
- damage helper

Change enemy spawn bundle from:

```rust
(Enemy, AggroRange::default(), EnemyAttack::default())
```

to something like:

```rust
(Enemy, AggroRange::default(), Spellbook::single(AttackSpell::ID), SpellCooldowns::default())
```

Add an enemy AI spell casting system, either in `src/plugins/spells/systems.rs` or enemy systems.

Recommended ownership:

- Enemy movement/chase stays in `plugins/entity/enemy/systems.rs`.
- Enemy decision to request Attack can be in enemy systems if it is AI-specific.
- Actual spell execution stays in `spells`.

Possible flow:

```text
enemy_auto_cast_attack system
        |
        v
SpellCastRequest { caster: enemy, spell_id: Attack }
        |
        v
SpellsPlugin cast system
        |
        v
AttackSpell::cast
```

This keeps AI and spell effects separate.

Validation:

```bash
cargo test enemy
cargo test spells
cargo check
```

### Slice 7 — Update network protocol

Remove from `src/network/protocol.rs`:

```rust
use crate::plugins::entity::enemy::components::EnemyAttack;
app.component::<EnemyAttack>().replicate();
```

Update imports:

```rust
use crate::stats::components::{CombatStats, MovementStats, VitalStats};
```

Decide whether to replicate spell data:

#### Minimum initial version

Do not replicate spell cooldowns/configs. Clients only need replicated `MovementStats`, `CombatStats`, and `VitalStats` to see results.

#### Debug/version with area indicators

Replicate a lightweight component such as:

```rust
pub struct ActiveSpellDebugArea {
    pub radius: f32,
}
```

or keep client-only visual debug generated from replicated spellbook/config if needed.

For now, because the current `EnemyAttack` was replicated only for visual debug, remove that dependency and either:

- temporarily remove the red area indicator, or
- recreate it from `Spellbook` + local built-in registry on the client.

Validation:

```bash
cargo check
```

### Slice 8 — Update UI and movement call sites

Update movement to read `MovementStats.speed`.

Update player stats UI to read:

- `VitalStats.max_health`
- `VitalStats.max_mana`
- `VitalStats.mana_regeneration`
- `CombatStats.armor`
- `CombatStats::armor_damage_reduction()` or an equivalent formula helper

Update imports from `plugins::entity::components` to `stats::components`.

Update health bar UI so the fill percentage is computed as:

```rust
vital.current_health / vital.max_health.max(0.1)
```

Validation:

```bash
cargo test ui
cargo check
```

### Slice 9 — Cleanup old attack system completely

Delete or rewrite old files/sections:

- Remove `EnemyAttack` from `enemy/components.rs`.
- Remove attack systems from `enemy/systems.rs`.
- Remove attack debug module dependency on `EnemyAttack`.
- Remove `EnemyAttack` protocol replication.
- Remove combat formulas from old `Stats` implementation.
- Remove obsolete tests from old modules after porting them.

Search must return no old attack implementation references:

```bash
rg "EnemyAttack|EnemyAttackCooldown|enemy_area_attack|initialize_attack_cooldowns"
```

Expected result: no matches except possibly migration notes or this plan.

Validation:

```bash
cargo fmt --check
cargo test
cargo check
```

---

## Target File Ownership After Refactor

| Concern | Target owner |
|---|---|
| Entity identity (`GameEntity`, `EntityState`, `PlayerName`) | `src/plugins/entity/components.rs` |
| Runtime stat components (`MovementStats`, `CombatStats`, `VitalStats`) | `src/stats/components.rs` |
| Aggregate stat DTO (`StatsBundleData`) | `src/stats/components.rs` or a nearby `bundle_data.rs` if it grows |
| Stat formulas | `src/stats/formulas.rs` |
| Damage/heal/modifier application | `src/stats/systems.rs` |
| Stat events (`DamageEvent`, `HealEvent`, `ApplyStatModifierEvent`) | `src/stats/events.rs` |
| Player/enemy default stat profiles | `src/stats/defaults.rs` or entity spawn modules using `StatsBundleData` |
| Spell trait and registry | `src/plugins/spells/registry.rs` |
| Spell runtime components | `src/plugins/spells/components.rs` |
| Spell cast events | `src/plugins/spells/events.rs` |
| Generic spell cast processing | `src/plugins/spells/systems.rs` |
| Concrete spell catalog | `src/spells/` |
| Attack spell behavior | `src/spells/attack/definition.rs` |
| Generic enemy framework, future | `src/plugins/enemies/` |
| Concrete enemy catalog, future | `src/enemies/` |
| Enemy chase AI | currently `src/plugins/entity/enemy/systems.rs`, later `src/plugins/enemies/systems.rs` |
| Enemy spell decision AI | currently `src/plugins/entity/enemy/systems.rs` or `src/plugins/spells/systems.rs`, but not inside `AttackSpell` |
| Network replication registration | `src/network/protocol.rs` |
| Player stats persistence | `src/persistence/entity/player_stats.rs` + `src/persistence/repository/player.rs` |

---

## Compatibility and Migration Notes

### Server authority

Keep all gameplay-affecting spell execution server-only:

```rust
.run_if(crate::network::mode::has_server)
```

Clients may display debug indicators, but they must not apply damage.

### Prediction

Movement currently depends on predicted stat state. Prediction should continue to work by replicating/predicting the split stat components.

In `ProtocolPlugin`, keep the equivalent of:

```rust
app.component::<MovementStats>().replicate().predict();
app.component::<CombatStats>().replicate().predict();
app.component::<VitalStats>().replicate().predict();
```

At minimum, `MovementStats` must remain predicted on clients so click-to-move prediction keeps working.

### Persistence defaults

When a new player is created, use the same current defaults unless intentionally changed:

Player:

```text
MovementStats.speed = 0.15
CombatStats.attack_power = 10.0
CombatStats.armor = 25.0
VitalStats.current_health = 100.0
VitalStats.max_health = 100.0
VitalStats.max_mana = 100.0
VitalStats.mana_regeneration = 5.0
```

Enemy:

```text
MovementStats.speed = 0.08
CombatStats.attack_power = 18.0
CombatStats.armor = 10.0
VitalStats.current_health = 50.0
VitalStats.max_health = 50.0
VitalStats.max_mana = 40.0
VitalStats.mana_regeneration = 2.0
```

Attack spell:

```text
cooldown_seconds = 1.0
area_radius = 3.0
```

### Death handling

Current generic death cleanup exists in `src/plugins/entity/systems.rs` but `EntityPlugin` does not currently register it in the inspected code.

During the refactor, decide explicitly whether death means:

1. Immediate despawn when `VitalStats.current_health <= 0`, or
2. Set `EntityState::Dead`, then another system despawns or respawns later.

Recommended first implementation:

- Stats system emits or applies death by setting `EntityState::Dead`.
- Existing cleanup/despawn behavior should be registered only if immediate removal is desired.
- Avoid hiding this decision inside `AttackSpell`.

---

## Risks and Mitigations

### Risk: Dynamic spell trait becomes too powerful and hard to schedule

Mitigation:

- Do not pass `&mut World` directly to spells initially.
- Use a constrained `SpellCastContext` and effect events such as `DamageEvent`, `HealEvent`, and `ApplyStatModifierEvent`.

### Risk: Registry IDs are hard to serialize

Mitigation:

- Use a small string-backed `SpellId` or enum-like newtype.
- Ensure it derives `Serialize`, `Deserialize`, `Clone`, `Eq`, and `Hash`.

### Risk: Lightyear replication of complex spell components is painful

Mitigation:

- Keep cooldowns server-only.
- Replicate only simple components when needed.
- Replicated stat components are enough for visible gameplay effect.

### Risk: DB migration breaks existing local databases

Mitigation:

- Use `if_not_exists` for the new table.
- Backfill existing players with default stats.
- Do not remove existing `players` columns.

### Risk: Too many changes in one PR/slice

Mitigation:

- Move Stats first without behavior changes.
- Add persistence second.
- Add spell infrastructure third.
- Replace Attack last.

---

## Acceptance Criteria

The refactor is complete when:

1. The old embedded stat definitions no longer live in `src/plugins/entity/components.rs`.
2. A dedicated `StatsPlugin` exists and is registered.
3. Runtime ECS uses `MovementStats`, `CombatStats`, and `VitalStats`.
4. `StatsBundleData` exists for spawn/config/persistence boundaries.
5. `DamageEvent`, `HealEvent`, and `ApplyStatModifierEvent` exist and are processed by `StatsPlugin` systems.
6. Player stats are loaded from PostgreSQL on join.
7. Player stats are saved to PostgreSQL on disconnect.
8. `src/plugins/spells/` contains the spell trait, registry, plugin, components, events, context, and systems; `src/spells/` contains the concrete `Attack` spell catalog entry.
9. The current enemy attack behavior works through the spell system, not through `EnemyAttack`.
10. `EnemyAttack`, `EnemyAttackCooldown`, `enemy_area_attack`, and `initialize_attack_cooldowns` are removed.
11. Network protocol no longer registers `EnemyAttack` and instead replicates/predicts the new split stat components.
12. Movement still uses replicated/predicted movement speed.
13. UI still displays player stats and health bars correctly.
14. The codebase is ready for a future workspace proc-macro crate under `crates/gameplay_macros/`, but the refactor does not depend on custom spell macros.
15. `cargo fmt --check`, `cargo test`, and `cargo check` pass.

---

## Suggested Implementation Order Summary

1. Introduce `MovementStats`, `CombatStats`, `VitalStats`, and `StatsBundleData` under `src/stats/`.
2. Register `StatsPlugin`.
3. Add `DamageEvent`, `HealEvent`, and `ApplyStatModifierEvent` plus the related systems.
4. Update imports and protocol registration for the split stat components.
5. Add DB table/entity/repository support for player stats through `StatsBundleData`.
6. Load stats on join and save stats on disconnect.
7. Create `src/plugins/spells/` framework infrastructure and `src/spells/` content catalog.
8. Implement `AttackSpell` under `src/spells/attack/` using current attack constants.
9. Replace enemy attack systems with spell cast requests.
10. Remove old `EnemyAttack` and old debug dependency.
11. Optionally reshape the repository as a workspace and reserve `crates/gameplay_macros/` for future shared proc macros.
12. Run full validation.
