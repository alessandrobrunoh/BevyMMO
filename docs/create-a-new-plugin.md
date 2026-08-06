# Create a new Entity Plugin

# Create a new Entity Plugin

Guide to adding a new game entity (Player, Enemy, NPC, Boss, ...)
inside the runtime package at `bins/game/src/plugins/entity/`.

Note: the workspace is now split. Shared data/traits such as `EntityDefinition`,
`GameEntityBundle`, replicated components, and stats DTOs live in
`crates/shared/`; server-authoritative systems progressively move to
`crates/server/`. This guide still uses the runtime package paths because the
compatibility facades remain there during the migration.

---

## TL;DR

To create a new `Foo` entity that is **automatically synchronized over the
network**:

1. Create `bins/game/src/plugins/entity/foo/{mod.rs,components.rs,spawn.rs,systems.rs}`.
2. Define a `Foo` marker in `components.rs`.
3. Implement `EntityDefinition` in `spawn.rs`.
4. Register systems in `FooPlugin` (`mod.rs`).
5. Add `pub mod foo;` + `app.add_plugins(foo::FooPlugin);` in `entity/mod.rs`.

That's it: the new entity will have `GameEntity`, `Health`, `Position`,
`EntityColor`, `Replicate` (configured by `GameEntityBundle`) without touching
network code.

---

## Architecture

Each concrete entity is a **sub-module** of `bins/game/src/plugins/entity/` with this
fixed structure:

```
bins/game/src/plugins/entity/
├── mod.rs              # EntityPlugin (parent) → registers all child FooPlugins
├── components.rs       # GameEntity, Health (shared by all)
├── definition.rs       # EntityDefinition trait (entity defaults)
├── spawn.rs            # GameEntityBundle + spawn_entity::<T>()
├── systems.rs          # shared systems (With<GameEntity>)
└── <name>/
    ├── mod.rs          # <Name>Plugin (Bevy Plugin) — registers systems
    ├── components.rs   # Marker component + specific data components
    ├── spawn.rs        # impl EntityDefinition for <Name>
    └── systems.rs      # entity-specific systems
```

The parent plugin `EntityPlugin` (in `entity/mod.rs`) automatically registers
all child plugins. `EntityPlugin` is already registered in `main.rs` (in all
modes: client/server/host), so a new entity requires no changes in `main.rs`.

### Key Concepts

| Concept | Location | Purpose |
|---|---|---|
| `GameEntity` | `entity/components.rs` | Filter all game entities with `With<GameEntity>` |
| `Health` | `entity/components.rs` | Shared health; used by `despawn_dead_entities` |
| `EntityDefinition` trait | `entity/definition.rs` | Contract for defaults and specific bundle |
| `GameEntityBundle` | `entity/spawn.rs` | Shared state: marker, health, stats, position, color and replication |
| `spawn_entity::<T>()` | `entity/spawn.rs` | Helper for standard enemy/NPC spawning |
| `Position`, `EntityColor` | `network/protocol.rs` | Replicated network components (generic, not bound to player) |
| `<Name>Plugin` | `<name>/mod.rs` | Bevy `Plugin` registering specific systems |

### Automatic Network: How it Works

Network components are **generic** and registered once in
`network/protocol.rs`:

```rust
app.component::<Position>().replicate().predict().add_linear_interpolation();
app.component::<EntityColor>().replicate();
```

When you call `spawn_entity::<T>()`, the helper adds `Replicate::to_clients(...)`
to the entity, so lightyear automatically replicates `Position` and `EntityColor`
to all clients. The new entity does not need to register anything new on the network.

---

## Step-by-step: adding `Npc`

### 1. Create directory and the 4 files

```
src/plugins/entity/npc/
├── mod.rs
├── components.rs
├── spawn.rs
└── systems.rs
```

### 2. `components.rs` — marker + data

```rust
use bevy::prelude::*;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Npc;

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct DialogueTree(pub &'static str);
```

### 3. `spawn.rs` — implement `EntityDefinition`

```rust
use bevy::prelude::*;

use super::components::{DialogueTree, Npc};
use crate::plugins::entity::{components::Health, definition::EntityDefinition};

impl EntityDefinition for Npc {
    fn name() -> &'static str { "Npc" }

    fn bundle() -> impl Bundle {
        (Npc, DialogueTree("hello"))
    }

    fn initial_position() -> Vec3 { Vec3::new(-3.0, 0.0, 2.0) }
    fn initial_color() -> Color { Color::srgb(0.6, 0.6, 0.2) }
    fn health() -> Health { Health::new(30.0) }
}
```

All methods except `name()` and `bundle()` are optional (they have defaults).

### 4. `systems.rs` — specific logic (server-authoritative if moving Position)

```rust
use bevy::prelude::*;
use super::components::*;

pub fn npc_idle(/* query, ... */) {
    // ...
}
```

If the system moves `Position` (server-authoritative movement) it should be registered with
a server-only guard (see step 5). If it is client-only (UI, fx), `Update` is fine.

### 5. `mod.rs` — the Bevy Plugin

```rust
pub mod components;
pub mod spawn;
pub mod systems;

use bevy::prelude::*;

pub use components::Npc;

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, systems::npc_idle);
        // For server-authoritative systems:
        // app.add_systems(FixedUpdate, systems::npc_ai
        //     .run_if(crate::network::mode::has_server));
    }
}
```

### 6. Register in parent plugin

In `src/plugins/entity/mod.rs`:

```rust
pub mod npc;                                    // <-- add

impl Plugin for EntityPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(enemy::EnemyPlugin);
        app.add_plugins(npc::NpcPlugin);        // <-- add
    }
}
```

### 7. Spawn it wherever you want

```rust
use crate::plugins::entity::spawn::spawn_entity;
use crate::plugins::entity::npc::components::Npc;

// In any system with `Commands`:
let npc = spawn_entity::<Npc>(&mut commands);
```

`Position`, `EntityColor`, `Health`, `GameEntity`, `Npc`, `Replicate` are
already all on the entity.

---

## Special cases

### Player (custom network)

The Player has prediction/interpolation dependent on owner (`PeerId`), which
cannot live in `spawn_entity::<T>()`. Player spawning therefore remains in the
`network::server::handle_connected_client` system, but still uses
`Player::bundle()` and `Player::health()` from `EntityDefinition` to stay
consistent:

```rust
commands.spawn((
    GameEntityBundle::new(
        Position(Vec3::ZERO),
        EntityColor(color),
        Player::health(),
        Player::stats(),
        NetworkTarget::All,
    ),
    Player::bundle(),
    PlayerId(peer_id),
    PredictionTarget::to_clients(NetworkTarget::Single(peer_id)),
    InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(peer_id)),
    ControlledBy { owner: ..., lifetime: ... },
    ActionState::<Inputs>::default(),
));
```

### Server-only systems

For systems that compute server-authoritative movement (AI, simulation),
use:

```rust
app.add_systems(
    FixedUpdate,
    foo_ai.run_if(crate::network::mode::has_server),
);
```

This way they don't run on clients (where `Position` already arrives replicated via
lightyear).

---

## Conventions

- **Network components** (`Position`, `EntityColor`, ...) remain in
  `src/network/protocol.rs`. Do NOT define them inside `entity/<name>/`.
- **Identity-specific components** (`Player`, `Enemy`, `AggroRange`, ...)
  live in `<name>/components.rs`.
- **Marker components** always derive `Component, Debug, Default, Clone, Copy`.
- **Data components with `Reflect`** use `#[reflect(Component)]`.
- **One plugin = one entity**. If an entity has variants, use an enum component
  or an additional marker, not multiple plugins.
- **Shared systems** (operating on all entities) go into
  `entity/systems.rs` with `With<GameEntity>`.
- **Specific systems** of an entity go into `<name>/systems.rs` with the
  entity marker (`With<Player>`, `With<Enemy>`, ...).

## When NOT to use this structure

If the feature is not a game entity (e.g. `RendererPlugin`, `UiPlugin`,
`AudioPlugin`), do not put it in `entity/`. Create a file or folder directly
in `src/plugins/` and register it in `main.rs`.
