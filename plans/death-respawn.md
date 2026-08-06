# Plan: death, death UI, Player respawn and Enemy respawn

## Goal

Replace the current behavior (immediate despawn of any entity when `VitalStats.is_dead()`) with a full death/respawn flow:

1. **Death** — when `current_health` reaches 0, the entity enters `EntityState::Dead` instead of being despawned. `Position`, `VitalStats`, `EntityKind` and the rest stay replicated so UI and visual feedback keep working.
2. **Death UI (local Player)** — a full-screen overlay ("You died") with a `Respawn` button that sends a request to the server. Client-only.
3. **Player respawn** — happens server-side, triggered by the client. The player is moved back to its spawn point with regenerated stats.
4. **Enemy respawn** — every dead `Enemy` respawns automatically after `10s`, recreated at its original spawn position with regenerated stats. The targeted marker is `Enemy` (NOT the generic Bevy `Entity`); the wording "Entity Enemy (call it that, not just Entity)" means the gameplay identifier is the `Enemy` marker component, not `bevy::ecs::entity::Entity`.

All gameplay-mutating logic (transition to `Dead`, HP regeneration, enemy recreation) stays **server-authoritative** and is replicated to clients through already-replicated components (`EntityState`, `VitalStats`, `Position`).

## Current state

- `src/plugins/entity/systems.rs::despawn_dead_entities` despawns any entity whose `VitalStats.is_dead()` as soon as health changes. **To be replaced**: we don't want despawn anymore, we want to set `EntityState::Dead`.
- `src/plugins/entity/components.rs::EntityState` already has a `Dead` variant (terminal until an explicit respawn system changes it) — use it.
- `EntityKind`, `Position`, `VitalStats`, `EntityState` are already replicated via `ProtocolPlugin` in `src/network/protocol.rs`.
- `Enemy` (marker) and `AggroRange` live in `src/plugins/entity/enemy/components.rs`; the spawn definition is `src/plugins/entity/enemy/spawn.rs` (initial position `Vec3::new(5.0, 0.0, 5.0)`, stats from `enemy_defaults()`).
- The local player is identified by `Controlled` or by `PlayerId == client_id` (see `src/ui/player_stats/systems.rs::update_player_stats`).
- The enemy AI systems in `enemy/systems.rs` (`enemy_chase`, `enemy_auto_cast_attack`) don't filter by state: a dead enemy would keep aggroing — must be blocked while in `EntityState::Dead`.
- UI plugins are exposed in `src/ui/plugin.rs`; each panel lives in its own folder (`player_stats`, `entity_bar`, `target_frame`, ...). The new panel follows the same layout.
- `src/game_state.rs::GameScreen/Screen` already has `InGame`/`Paused` to use as UI execution condition.

## Design

### Death flow

```text
Server: apply_damage drives current_health to 0
  -> mark_dead_entities system (FixedUpdate, has_server)
     iterates Query<&mut EntityState, &VitalStats, Changed<VitalStats>>
     if vital.is_dead() && *state != Dead  ->  *state = Dead
     emits DeathEvent { entity, kind } (for logging/UI/aggro drop)
  -> EntityState = Dead is replicated to clients
```

### Player respawn flow

```text
Client (local player): "Respawn" pressed
  -> sends RespawnRequest (Channel2, client -> server)
Server:
  -> handle_respawn_request system (FixedUpdate, has_server)
     for each RespawnRequest:
       - resolve entity from sender peer
       - require EntityState == Dead (otherwise drop)
       - reset Position to the player spawn point
       - reset VitalStats (current_health = max_health, full mana)
       - EntityState = Idle
       - emit RespawnedEvent
  -> changes are replicated to clients
Client: the death UI hides when it sees EntityState != Dead
```

### Enemy respawn flow

```text
Server: enemy_respawn system (Update, has_server)
  for every Enemy (Entity, &EntityState) that just entered Dead:
    - insert component Respawning { remaining: 10.0 }
  for every Enemy with Respawning:
    - remaining -= delta
    - if remaining <= 0:
        - reset Position to the original spawn
        - reset VitalStats to enemy_defaults()
        - EntityState = Idle
        - remove Respawning
  -> everything is replicated to clients
```

The original spawn position must be remembered. Add a `SpawnPoint(Vec3)` component attached to every entity at spawn time (server-side) and replicated. Useful in the future also for player respawn variants.

## Implementation steps

### 1. Death/respawn events (new)

File: `src/plugins/entity/events.rs` (new)

```rust
#[derive(Event, Clone, Copy, Debug)]
pub struct DeathEvent {
    pub entity: Entity,
    pub kind: EntityKind,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct RespawnedEvent {
    pub entity: Entity,
}
```

Exported from `src/plugins/entity/mod.rs` and registered in `EntityPlugin` with `app.add_event::<DeathEvent>()` and `app.add_event::<RespawnedEvent>()`.

### 2. `SpawnPoint` component

File: `src/plugins/entity/components.rs`

```rust
#[derive(Component, Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq)]
#[reflect(Component)]
pub struct SpawnPoint(pub Vec3);
```

Register in `ProtocolPlugin` with `replicate()` and in `EntityPlugin` with `register_type`. Add it to `GameEntityBundle` in `src/plugins/entity/spawn.rs` using `T::initial_position()`.

### 3. Replace `despawn_dead_entities` with `mark_dead_entities`

File: `src/plugins/entity/systems.rs`

- Remove `despawn_dead_entities`.
- Add:

```rust
pub fn mark_dead_entities(
    mut death_events: EventWriter<DeathEvent>,
    query: Query<(Entity, &mut EntityState, &VitalStats, &EntityKind), Changed<VitalStats>>,
) {
    for (entity, mut state, vital, kind) in query.iter() {
        if vital.is_dead() && *state != EntityState::Dead {
            *state = EntityState::Dead;
            death_events.send(DeathEvent { entity, kind: *kind });
            info!("Entity {:?} ({:?}) died", entity, kind);
        }
    }
}
```

- Move the registration from `PlayerPlugin` (in `player/mod.rs`) to `EntityPlugin` (in `entity/mod.rs`), in `FixedUpdate` with `run_if(has_server)`. This way it applies to all entities, not just the player.

### 4. AI systems: stop dead enemies

File: `src/plugins/entity/enemy/systems.rs`

- Add an `EntityState != Dead` filter in the `enemy_chase` and `enemy_auto_cast_attack` queries. Cleaner option: read `&EntityState` in the query and skip `Dead` entries inline, instead of introducing a parallel marker component.

### 5. `Respawning` component + Enemy respawn system

File: `src/plugins/entity/enemy/components.rs`

```rust
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct Respawning {
    pub remaining: f32,
}

/// Centralized constant for the enemy respawn time.
pub const ENEMY_RESPAWN_SECONDS: f32 = 10.0;
```

File: `src/plugins/entity/enemy/systems.rs` — add:

```rust
pub fn schedule_enemy_respawn(
    mut commands: Commands,
    just_dead: Query<(Entity, &EntityState), (With<Enemy>, Added<EntityState>)>,
) {
    for (entity, state) in just_dead.iter() {
        if *state != EntityState::Dead {
            continue;
        }
        commands.entity(entity).insert(Respawning { remaining: ENEMY_RESPAWN_SECONDS });
    }
}

pub fn enemy_respawn(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut Respawning,
        &mut Position,
        &mut VitalStats,
        &mut EntityState,
        &SpawnPoint,
    )>,
) {
    let delta = time.delta().as_secs_f32();
    for (entity, mut respawning, mut position, mut vital, mut state, spawn) in query.iter_mut() {
        respawning.remaining -= delta;
        if respawning.remaining > 0.0 {
            continue;
        }
        position.0 = spawn.0;
        vital.current_health = vital.max_health;
        vital.clamp_health();
        *state = EntityState::Idle;
        commands.entity(entity).remove::<Respawning>();
        info!("Enemy {:?} respawned at {:?}", entity, spawn.0);
    }
}
```

Register both in `EnemyPlugin` on `FixedUpdate` with `run_if(has_server)`, for consistency with the existing server-side AI systems.

### 6. Client -> server respawn command

File: `src/network/protocol.rs`

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RespawnRequest;
```

Register in `ProtocolPlugin` on `Channel2` with direction `ClientToServer` (existing reliable channel, already used by `JoinRequest` and `SpellCastCommand`):

```rust
app.register_message::<RespawnRequest>()
    .add_direction(NetworkDirection::ClientToServer);
```

### 7. Server-side Player respawn handler

File: `src/network/server.rs` (where `JoinRequest` and `SpellCastCommand` are already handled)

- Add a `handle_respawn_request` system that reads `MessageReader<RespawnRequest>`, resolves the sender peer's player (same pattern as `handle_join_request` / `handle_spell_cast`), and if `EntityState == Dead`:
  - reset `Position` to the player spawn point (`SpawnPoint` or fallback `Vec3::ZERO`)
  - reset `VitalStats.current_health = max_health`, mana = `max_mana`
  - `EntityState = Idle`
  - emit `RespawnedEvent`

Define the player spawn position in a single place (e.g. constant `PLAYER_SPAWN_POINT: Vec3 = Vec3::ZERO` in `src/plugins/entity/player/spawn.rs`, reused by `initial_position()`).

### 8. Client: sending `RespawnRequest`

File: `src/ui/death_screen/systems.rs` (new UI module) — see section 9.

The "Respawn" button system emits `RespawnRequest` via `MessageWriter` when the local player is `Dead` and the user clicks.

### 9. DeathScreen UI (new)

Structure consistent with existing UI plugins (`player_stats`, `pause_menu`):

```
src/ui/death_screen/
├── mod.rs
├── plugin.rs
└── systems.rs
```

`src/ui/death_screen/plugin.rs`:

```rust
#[derive(Component)] pub struct DeathScreenRoot;
#[derive(Component)] pub struct DeathScreenButton;

pub struct DeathScreenPlugin;

impl Plugin for DeathScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_death_screen);
        app.add_systems(Update, (
            update_death_screen_visibility,
            handle_respawn_button,
        ).run_if(has_client));
    }
}
```

`setup_death_screen`: spawn a full-screen `Node` (Display::None by default), background semi-transparent `screen_bg`, containing:
- Title "You died" (font size `title_font_size`).
- Optional subtitle "Press Respawn to go back into the game".
- "Respawn" button (`DeathScreenButton`) styled using the theme buttons.

`update_death_screen_visibility`:
- Read the local player's `EntityState` (`Controlled` or `PlayerId == client_id`, same as `update_player_stats`).
- `root.display = Display::Flex` if `state.is_dead()` and `Screen::InGame | Paused`, otherwise `Display::None`.

`handle_respawn_button`:
- On interaction (`OnPress` / `Interaction::Pressed`) of the `DeathScreenButton`, write a `RespawnRequest` via `MessageWriter`.
- Disable further clicks until state changes (to avoid spam).

Registration: add `death_screen::DeathScreenPlugin` to `src/ui/plugin.rs::UiPlugin` alongside the others.

### 10. Module exports

- `src/plugins/entity/mod.rs`: add `pub mod events;`, export `DeathEvent` / `RespawnedEvent`, register events and `mark_dead_entities` in `EntityPlugin`.
- `src/ui/mod.rs`: add `pub mod death_screen;`.

## Key point: the "Enemy" name

The user explicitly asks to treat `Enemy` (the marker component in `src/plugins/entity/enemy/components.rs`) as the respawn target, NOT the generic `bevy::ecs::entity::Entity`. The flow therefore always filters with `With<Enemy>`. The death UI instead concerns the **local Player** (`Player` marker + `Controlled`/`PlayerId`), not the Enemy.

## Tests

### Unit/integration tests

- `entity/systems.rs`: `mark_dead_entities` sets `EntityState::Dead` and emits `DeathEvent` when `VitalStats.is_dead()`, ignores entities already `Dead` or alive.
- `enemy/systems.rs`:
  - `schedule_enemy_respawn` inserts `Respawning` only for `Enemy` that just entered `Dead`.
  - `enemy_respawn` regenerates HP, restores `Position` to `SpawnPoint`, removes `Respawning` and resets `EntityState::Idle` when the timer expires.
- `enemy_chase` / `enemy_auto_cast_attack` ignore enemies with `EntityState::Dead`.
- `death_screen/systems.rs`:
  - `update_death_screen_visibility` shows the panel only when the local player is `Dead` and the screen is `InGame`/`Paused`.
  - `handle_respawn_button` sends `RespawnRequest` only on press.
- `network/server.rs` (if testable with a light mock): `handle_respawn_request` resets only `Dead` players and ignores requests for alive players.

### Manual validation (HostClient)

1. `cargo check` and `cargo test`.
2. Start HostClient + a second client.
3. Let the player get killed by an enemy: the death screen appears; the player stays visible on the ground (`EntityState::Dead`) on both clients.
4. Click "Respawn": the local player comes back to life at the spawn point with full HP, the UI updates on both clients.
5. Kill an enemy: after 10s it reappears at the spawn point with full HP and resumes aggro.

## Files to modify/create

**Modifications**

- `src/plugins/entity/mod.rs` — expose `events`, register events and `mark_dead_entities`, register `SpawnPoint` as reflected type.
- `src/plugins/entity/components.rs` — add `SpawnPoint`.
- `src/plugins/entity/systems.rs` — replace `despawn_dead_entities` with `mark_dead_entities`.
- `src/plugins/entity/spawn.rs` — add `SpawnPoint` to `GameEntityBundle`.
- `src/plugins/entity/player/mod.rs` — remove registration of `despawn_dead_entities`.
- `src/plugins/entity/enemy/mod.rs` — register `schedule_enemy_respawn` and `enemy_respawn`.
- `src/plugins/entity/enemy/components.rs` — add `Respawning` and `ENEMY_RESPAWN_SECONDS`.
- `src/plugins/entity/enemy/systems.rs` — add `schedule_enemy_respawn` and `enemy_respawn`, filter `Dead` in existing AI systems.
- `src/plugins/entity/player/spawn.rs` — expose `PLAYER_SPAWN_POINT` constant and use it in `initial_position()`.
- `src/network/protocol.rs` — register `SpawnPoint` as replicated + `RespawnRequest` message.
- `src/network/server.rs` — add `handle_respawn_request`.
- `src/ui/mod.rs` — add `pub mod death_screen;`.
- `src/ui/plugin.rs` — register `DeathScreenPlugin`.

**New**

- `src/plugins/entity/events.rs`
- `src/ui/death_screen/mod.rs`
- `src/ui/death_screen/plugin.rs`
- `src/ui/death_screen/systems.rs`

## Suggested follow-ups (out of scope)

- Enemy loot drop on death.
- Death penalty (experience, items, growing respawn cooldown).
- Animations/mesh for the `Dead` state (e.g. entity lying down or fade-out).
- Player respawn with temporary invulnerability.
- Multiple spawn points per enemy (database/resource) instead of the single `initial_position()`.
- Persistence: save the alive/dead state of the player to DB on logout.
