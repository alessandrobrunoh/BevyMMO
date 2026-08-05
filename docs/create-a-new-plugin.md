# Create a new Entity Plugin

Guida per aggiungere una nuova entità di gioco (Player, Enemy, NPC, Boss, ...)
all'interno del modulo `src/plugins/entity/`.

---

## TL;DR

Per creare una nuova entità `Foo` che sia **automaticamente sincronizzata sul
network**:

1. Crea `src/plugins/entity/foo/{mod.rs,components.rs,spawn.rs,systems.rs}`.
2. Definisci un marker `Foo` in `components.rs`.
3. Implementa `EntityDefinition` in `spawn.rs`.
4. Registra i sistemi nel `FooPlugin` (`mod.rs`).
5. Aggiungi `pub mod foo;` + `app.add_plugins(foo::FooPlugin);` in `entity/mod.rs`.

Tutto qui: la nuova entità avrà `GameEntity`, `Health`, `Position`,
`EntityColor`, `Replicate` (configurati da `GameEntityBundle`) senza toccare
il codice di rete.

---

## Architettura

Ogni entità concreta è un **sotto-modulo** di `src/plugins/entity/` con questa
struttura fissa:

```
src/plugins/entity/
├── mod.rs              # EntityPlugin (padre) → registra tutti i FooPlugin figli
├── components.rs       # GameEntity, Health (condivise da tutte)
├── definition.rs       # trait EntityDefinition (defaults dell'entità)
├── spawn.rs            # GameEntityBundle + spawn_entity::<T>()
├── systems.rs          # sistemi condivisi (With<GameEntity>)
└── <name>/
    ├── mod.rs          # <Name>Plugin (Bevy Plugin) — registra sistemi
    ├── components.rs   # Marker component + componenti dati specifiche
    ├── spawn.rs        # impl EntityDefinition for <Name>
    └── systems.rs      # sistemi specifici dell'entità
```

Il plugin padre `EntityPlugin` (in `entity/mod.rs`) registra automaticamente
tutti i plugin figli. `EntityPlugin` è già registrato in `main.rs` (in tutte
le modalità: client/server/host), quindi una nuova entità non richiede modifiche
in `main.rs`.

### Concetti chiave

| Concetto | Dove | A cosa serve |
|---|---|---|
| `GameEntity` | `entity/components.rs` | Filtrare tutte le entità di gioco con `With<GameEntity>` |
| `Health` | `entity/components.rs` | Salute condivisa; usata da `despawn_dead_entities` |
| `EntityDefinition` trait | `entity/definition.rs` | Contratto di defaults e bundle specifico |
| `GameEntityBundle` | `entity/spawn.rs` | Stato comune: marker, health, stats, position, color e replica |
| `spawn_entity::<T>()` | `entity/spawn.rs` | Helper per lo spawn standard di enemy/NPC |
| `Position`, `EntityColor` | `network/protocol.rs` | Componenti di rete replicate (generiche, non legate al player) |
| `<Name>Plugin` | `<name>/mod.rs` | Bevy `Plugin` che registra sistemi specifici |

### Network automatico: come funziona

Le componenti di rete sono **generic** e registrate una sola volta in
`network/protocol.rs`:

```rust
app.component::<Position>().replicate().predict().add_linear_interpolation();
app.component::<EntityColor>().replicate();
```

Quando chiami `spawn_entity::<T>()`, l'helper aggiunge `Replicate::to_clients(...)`
all'entità, quindi lightyear replica automaticamente `Position` ed `EntityColor`
a tutti i client. La nuova entità non deve registrare nulla di nuovo sul network.

---

## Step-by-step: aggiungere `Npc`

### 1. Crea la cartella e i 4 file

```
src/plugins/entity/npc/
├── mod.rs
├── components.rs
├── spawn.rs
└── systems.rs
```

### 2. `components.rs` — marker + dati

```rust
use bevy::prelude::*;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct Npc;

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct DialogueTree(pub &'static str);
```

### 3. `spawn.rs` — implementa `EntityDefinition`

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

Tutti i metodi tranne `name()` e `bundle()` sono opzionali (hanno un default).

### 4. `systems.rs` — logica specifica (server-authoritative se muove Position)

```rust
use bevy::prelude::*;
use super::components::*;

pub fn npc_idle(/* query, ... */) {
    // ...
}
```

Se il sistema muove `Position` (movimento server-authoritative) va registrato con
guard server-only (vedi step 5). Se è solo client-side (UI, fx), `Update` va bene.

### 5. `mod.rs` — il Bevy Plugin

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
        // Per sistemi server-authoritative:
        // app.add_systems(FixedUpdate, systems::npc_ai
        //     .run_if(crate::network::mode::has_server));
    }
}
```

### 6. Registra nel plugin padre

In `src/plugins/entity/mod.rs`:

```rust
pub mod npc;                                    // <-- aggiungi

impl Plugin for EntityPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(player::PlayerPlugin);
        app.add_plugins(enemy::EnemyPlugin);
        app.add_plugins(npc::NpcPlugin);        // <-- aggiungi
    }
}
```

### 7. Spawnalo dove vuoi

```rust
use crate::plugins::entity::spawn::spawn_entity;
use crate::plugins::entity::npc::components::Npc;

// In qualsiasi sistema con `Commands`:
let npc = spawn_entity::<Npc>(&mut commands);
```

`Position`, `EntityColor`, `Health`, `GameEntity`, `Npc`, `Replicate` sono
già tutti sull'entità.

---

## Casi speciali

### Player (network custom)

Il Player ha prediction/interpolation dipendenti dall'owner (`PeerId`), che non
possono stare in `spawn_entity::<T>()`. Lo spawn del player resta quindi nel
sistema `network::server::handle_connected_client`, ma usa comunque
`Player::bundle()` e `Player::health()` da `EntityDefinition` per restare
consistente:

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

### Sistemi server-only

Per sistemi che calcolano movimento server-authoritative (AI, simulazione),
usa:

```rust
app.add_systems(
    FixedUpdate,
    foo_ai.run_if(crate::network::mode::has_server),
);
```

Così non girano sui client (dove la `Position` arriva già replicata via
lightyear).

---

## Convenzioni

- **Componenti di rete** (`Position`, `EntityColor`, ...) restano in
  `src/network/protocol.rs`. NON definirle dentro `entity/<name>/`.
- **Componenti identità-specifiche** (`Player`, `Enemy`, `AggroRange`, ...)
  vivono in `<name>/components.rs`.
- **Marker components** derivano sempre `Component, Debug, Default, Clone, Copy`.
- **Componenti dati con `Reflect`** usano `#[reflect(Component)]`.
- **Un plugin = un'entità**. Se un'entità ha varianti, usa un enum component
  o un marker aggiuntivo, non più plugin.
- **Sistemi condivisi** (operano su tutte le entità) vanno in
  `entity/systems.rs` con `With<GameEntity>`.
- **Sistemi specifici** di un'entità vanno in `<name>/systems.rs` con il
  marker dell'entità (`With<Player>`, `With<Enemy>`, ...).

## Quando NON usare questa struttura

Se la feature non è un'entità di gioco (es. `RendererPlugin`, `UiPlugin`,
`AudioPlugin`), non metterla in `entity/`. Crea un file o cartella diretta
in `src/plugins/` e registrala in `main.rs`.
