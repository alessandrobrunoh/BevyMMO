# Plan: Workspace & Crate Split (server / client / presentation / editor)

**Status**: Draft — no code moved yet.
**Scope**: Convert the single-crate game into a Cargo workspace with a `shared` core crate plus `server`, `client`, `presentation` and `editor` crates, while preserving the current behavior of `cargo run -- client | server | host-client` exactly.

---

## Goal

The project currently is one fat crate (`bevy_lightyear_game`) where every module imports every other module via `crate::`. This works for a prototype but blocks the editor, the headless production server, and long-term compile times. This plan restructures the code into a workspace:

```
Cargo.toml                        # [workspace] root
crates/
├── shared/                       # types + protocol registration + world manifest (pure data)
├── server/                       # server-only: transport, persistence, authoritative systems
├── client/                       # client-only: transport, input, targeting
├── presentation/                 # rendering, scenes, game UI (Bevy UI)
└── editor/                       # map editor (bevy_egui + picking)
bins/
└── game/                         # thin binary: CLI -> composes plugins (stays the fat binary)
```

**Constraints (non-negotiable)**:

1. `cargo run -- client`, `cargo run -- server`, `cargo run -- host-client` keep working after **every** slice.
2. The production server build stays small: `cargo build --release --no-default-features --features server,netcode,udp,replication` must not compile `presentation`, `editor`, windowing, or game UI.
3. No behavior changes. No temporary hacks, no duplicated types, no "porting" bugs.
4. Game UI (`ui/`, Bevy UI native) is **not** rewritten. `bevy_egui` is only for the editor (see `plans/map-editor.md`).
5. `shared` never depends on `server`, `client`, `presentation`, or `editor`. Dependencies only point downward: `editor → presentation|client → shared` and `server → shared`.

---

## Current state (verified)

### Module inventory and dependencies today

| Current module | Imports from (`crate::`) | Target crate |
|---|---|---|
| `src/main.rs` | everything (bootstrap) | `bins/game` |
| `src/settings.rs` | none | `shared` (both binaries reuse config) |
| `src/game_state.rs` | none | **split**: `GameScreen`/`Screen` → `presentation`; `ConnectionRequest/Intent/Failure` → `client`; `validate_player_name` → `shared` |
| `src/migrations/` | sea-orm only | `server` |
| `src/network/mode.rs` | bevy only | `shared` (`AppMode`, `has_server`, `has_client`) |
| `src/network/protocol.rs` | `plugins/entity/components`, `plugins/spells`, `stats/components` | `shared` (types + `ProtocolPlugin`) |
| `src/network/client.rs` | `game_state`, `network/mode`, `plugins/spells`, `plugins/targeting` | `client` |
| `src/network/server.rs` | `game_state`, `plugins/entity/*`, `plugins/persistence`, `plugins/spells`, `stats/components` | `server` |
| `src/plugins/entity/components.rs` | bevy/serde/lightyear | `shared` |
| `src/plugins/entity/definition.rs` | `stats/components`, lightyear | `shared` |
| `src/plugins/entity/spawn.rs` | `network/protocol`, entity components, lightyear | `shared` (pure ECS composition) |
| `src/plugins/entity/events.rs` | — | `shared` |
| `src/plugins/entity/systems.rs` | entity components, `stats/components` | `server` (death/state transitions) |
| `src/plugins/entity/player/*` | components → `shared`; spawn → `shared`; systems → `server` | split per file |
| `src/plugins/entity/enemy/*` | components/spawn → `shared`; systems (AI) → `server`; `debug.rs` (visuals) → `presentation` | split per file |
| `src/plugins/entity/dummy/*` | data only | `shared` |
| `src/plugins/entity/boss/*` | components → `shared`; systems → `server`; `arena_visual.rs`/`dragon_visual.rs` → `presentation`; `ui/boss_bar/` → `presentation` | split per file |
| `src/plugins/spells/*` | data types + `Spell` trait + registry → `shared`; cast pipeline (`process_cast_requests`, `advance_cast_progress`, `fire_spell`, `aoe`, `effects`) → `server`; `cast_bar.rs`, HUD state → `presentation` | split per responsibility |
| `src/spells/*` (concrete spells) | `Spell` trait from `shared` | `shared` (implementations are data + `cast()` contracts) |
| `src/stats/components.rs` | bevy/serde | `shared` |
| `src/stats/{events,formulas,modifiers,defaults}.rs` | bevy | `shared` |
| `src/stats/{plugin,systems}.rs` | events/components | `server` (damage/heal/modifier application) |
| `src/plugins/persistence/*` | sea-orm, `plugins/persistence` only | `server` |
| `src/plugins/crowd_control/components.rs` | bevy | `shared` |
| `src/plugins/crowd_control/{mod,systems,events}.rs` | spells, `network/mode` | `server` |
| `src/plugins/targeting/` | `network/protocol` | `client` (systems) + `shared` (`CurrentTarget` type used by UI) |
| `src/plugins/key_mapping.rs` | bevy | `client` |
| `src/plugins/player_movement.rs` | `network/protocol` | **split**: client input/click → `client`; authoritative movement simulation → `server` |
| `src/plugins/renderer.rs` | `network/protocol`, `game_state` | `presentation` |
| `src/scenes/*` | `network/protocol`, `game_state`, renderer | `presentation` |
| `src/ui/*` | many | `presentation` |

### Key cross-cutting facts

- `Position`, `EntityColor`, `EntityKind`, `EntityState`, `GameEntity`, `SpawnPoint`, `PlayerName`, `Health`, `VitalStats`, `CombatStats`, `MovementStats`, `StatsBundleData`, `SpellId`, `HotbarSlot`, `SpellHotbar` are consumed by **both** network and gameplay modules: they all belong in `shared`.
- `ProtocolPlugin` (in `network/protocol.rs`) only **registers** components/messages with Lightyear; it opens no sockets. It can live in `shared` because registration is transport-agnostic.
- `spawn_entity::<T>()` and `GameEntityBundle` use `lightyear::prelude::{NetworkTarget, Replicate}` but no transport: they belong in `shared`.
- `AppMode`/`has_server`/`has_client` are imported by gameplay, network, UI and scenes: they belong in `shared`.
- The persistence plugin is server-only (SeaORM + PostgreSQL): it belongs in `server` and drags `tokio`, `sea-orm`, `migrations` with it.

---

## Design decisions

### D1. Dependency direction is strictly downward

```
presentation ──► client ──► shared
      │            ▲
      └────────────┘          (presentation may also depend directly on shared)
server ─────────────────► shared
editor ─────────────────► shared (+ presentation for the scene, if needed)
```

Rules enforced by `cargo check` + review:
- `shared` may only depend on `bevy`, `lightyear` (data features: `replication`), `serde`, `uuid`, `clap` (for settings types only if needed).
- `server` may NOT depend on `client`, `presentation`, `editor`.
- `presentation` may NOT depend on `server`.
- `editor` may NOT depend on `server`.

### D2. One root package remains the fat binary until the very end

Migration mechanics: the current root package stays at the repo root and keeps compiling at every step. Each slice extracts a crate and re-wires imports from `crate::x` to `bevymmo_x::...`. Only in the final slice does the root package move to `bins/game/`. This guarantees `cargo run -- server` keeps working continuously.

### D3. Granular split of mixed files, not whole-directory moves

Several files contain both shared data and role-specific systems in one file (e.g. `network/server.rs`, `plugins/entity/enemy/systems.rs`). The plan splits **per function**: data/components/traits → `shared`, server-authoritative systems → `server`, client visuals/input → `client`, UI/rendering → `presentation`. A function that is pure data never moves with a system that spawns entities.

### D4. Feature flags survive inside the game binary

The game binary keeps the existing feature set (`client`, `server`, `netcode`, `udp`, `interpolation`, `prediction`, `replication`, `input_native`) plus a new `editor` feature. Production server build stays `--no-default-features --features server,...` and never pulls `bevymmo_presentation` or `bevymmo_editor`.

### D5. Naming convention for crates

| Crate | Package name | Public prelude |
|---|---|---|
| shared | `bevymmo_shared` | `bevymmo_shared::prelude::*` |
| server | `bevymmo_server` | `bevymmo_server::prelude::*` |
| client | `bevymmo_client` | `bevymmo_client::prelude::*` |
| presentation | `bevymmo_presentation` | `bevymmo_presentation::prelude::*` |
| editor | `bevymmo_editor` | `bevymmo_editor::prelude::*` |

Each crate re-exports its root types so `main.rs` composes plugins without deep paths.

---

## Target workspace layout

```text
Cargo.toml                        # [workspace] + [workspace.dependencies]
crates/shared/
├── Cargo.toml
└── src/
    ├── lib.rs                    # prelude, re-exports
    ├── game_state.rs             # validate_player_name (only the shared part)
    ├── settings.rs
    ├── network/
    │   ├── mode.rs               # AppMode, has_server, has_client
    │   └── protocol.rs           # Position, EntityColor, ..., ProtocolPlugin
    ├── entity/
    │   ├── components.rs
    │   ├── definition.rs
    │   ├── spawn.rs              # GameEntityBundle, spawn_entity::<T>()
    │   └── events.rs
    ├── stats/
    │   ├── components.rs
    │   ├── events.rs
    │   ├── formulas.rs
    │   ├── modifiers.rs
    │   └── defaults.rs
    ├── spells/
    │   ├── mod.rs                # SpellId, CastKind, HotbarSlot, SpellHotbar, ...
    │   ├── trait.rs              # Spell trait (no transport)
    │   └── registry.rs
    ├── spells_impl/              # concrete spells (attack, fireball, boss kit, ...)
    ├── crowd_control/
    │   └── components.rs
    └── world/
        ├── mod.rs                # (empty placeholder — populated by map-editor plan)
        ├── manifest.rs
        └── loader.rs
crates/server/
├── Cargo.toml
└── src/
    ├── lib.rs                    # ServerPlugin groups
    ├── network/server.rs
    ├── migrations/
    ├── persistence/
    ├── gameplay/                 # entity systems, enemy AI, boss systems, player systems
    ├── spells/                   # cast pipeline, aoe, effects
    ├── stats/                    # damage/heal application systems
    ├── crowd_control/systems.rs
    ├── player_movement.rs        # authoritative simulation only
    └── world/                    # spawn-from-manifest, collision grid (map-editor plan)
crates/client/
├── Cargo.toml
└── src/
    ├── lib.rs                    # ClientPlugin groups
    ├── network/client.rs
    ├── input/key_mapping.rs
    ├── targeting/
    ├── player_movement.rs        # client input + click target selection
    └── world/asset_registry.rs   # kind -> .glb path (populated by map-editor plan)
crates/presentation/
├── Cargo.toml
└── src/
    ├── lib.rs                    # PresentationPlugin groups
    ├── game_state.rs             # GameScreen, Screen + lifecycle
    ├── renderer/
    ├── scenes/
    ├── ui/                       # everything currently in src/ui/
    └── spells/hud/               # cast_bar, HUD cooldowns
crates/editor/
├── Cargo.toml
└── src/
    ├── lib.rs                    # EditorPlugin
    ├── camera.rs
    ├── placement.rs
    ├── gizmo.rs
    ├── inspector.rs
    ├── palette.rs
    ├── tools.rs
    └── io.rs
bins/game/
├── Cargo.toml
└── src/main.rs                   # CLI + plugin composition (thin)
```

---

## Slice 0 — Safety net (do this first, no code moves)

**Value**: every subsequent refactor can be validated against a known-good baseline.

**Path**: server headless → client connects → player spawns → replica → renderer/UI.

**Acceptance criteria**:

- [ ] `cargo test` passes on current tree (record the exact suite + counts).
- [ ] `cargo clippy -- -D warnings` passes (record warnings baseline; do not start with violations).
- [ ] Manual smoke checklist documented: `server` + two `client` + movement + scoreboard + death/respawn.
- [ ] The headless server build (`--no-default-features --features server,netcode,udp,replication`) compiles today; record its wall time for comparison.

**RED**: write `plans/smoke-test-checklist.md` if it does not exist.

**GREEN**: nothing extracted yet — baseline only.

**Pattern**: baseline harness.

**Verification**: `cargo test`, `cargo clippy -- -D warnings`, manual smoke test.

---

## Slice 1 — Workspace skeleton (no behavior change, still one crate with code)

**Value**: establishes the workspace + dependency graph so later slices only move code.

**Path**: root `Cargo.toml` becomes `[workspace]` with `members = ["crates/shared", "crates/server", "crates/client", "crates/presentation", "crates/editor", "."]`; `[workspace.dependencies]` hoists every shared dependency; empty `crates/*` stubs created with `lib.rs`; current package gains `dependencies` on the new crates (empty for now).

**Acceptance criteria**:

- [ ] Root `Cargo.toml` declares the workspace and hoists: `bevy`, `lightyear`, `serde`, `clap`, `config`, `sea-orm`, `sea-orm-migration`, `tokio`, `uuid` into `[workspace.dependencies]`.
- [ ] `crates/shared`, `crates/server`, `crates/client`, `crates/presentation`, `crates/editor` exist with `Cargo.toml` + `lib.rs` and compile.
- [ ] Current package still builds and `cargo run -- server|client|host-client` behaves identically.
- [ ] `cargo metadata` shows the 6 members; no circular `path` dependencies.
- [ ] `cargo clippy -- -D warnings` clean.

**RED**: workspace manifests compile empty crates.

**GREEN**: minimal manifests; **no source code moved yet**.

**Pattern**: *Facade* for the workspace root.

**Verification**: `cargo check --workspace`, `cargo run -- server` smoke, `cargo clippy`.

---

## Slice 2 — Extract `shared` (types + protocol registration)

**Value**: the single most important boundary. Everything both client and server agree on lives here; it makes the manifest format and network protocol explicit.

**Path**: move (verbatim, then fix imports) all files listed in the target layout for `shared`:
- `network/mode.rs`, `network/protocol.rs`
- `plugins/entity/components.rs`, `definition.rs`, `spawn.rs`, `events.rs`
- `stats/components.rs`, `events.rs`, `formulas.rs`, `modifiers.rs`, `defaults.rs`
- spell data types + `Spell` trait + registry
- concrete spell implementations
- `crowd_control/components.rs`
- `game_state::validate_player_name`, `settings.rs`

Import rewrites: `crate::network::mode` → `bevymmo_shared::network::mode`, etc. Use `bevymmo_shared::prelude::*` in consuming crates.

**Acceptance criteria**:

- [ ] `bevymmo_shared` compiles standalone with **only** `bevy`, `lightyear` (data features), `serde`, `uuid`, `config`, `clap`.
- [ ] `Position`, `EntityKind`, `VitalStats`, `SpellId`, `AppMode` are importable from `bevymmo_shared` and no longer exist in the game crate.
- [ ] `ProtocolPlugin` still registers every component/message previously registered (test in `shared` asserts registration list).
- [ ] Server, client, and all systems still compile via re-exported paths.
- [ ] `cargo test`, `cargo clippy -- -D warnings`, smoke tests pass unchanged.
- [ ] `shared` contains **zero** `#[cfg(feature = "client")]` / `#[cfg(feature = "server")]` gating: it is role-agnostic by construction.

**RED**: unit test asserting `ProtocolPlugin` registers all expected components/messages; unit test that `shared` has no server/client features enabled.

**GREEN**: move data + trait files; fix imports mechanically (sed/grep for `crate::` prefixes per file); add `prelude` re-exports.

**Pattern**: *Data Transfer Objects* + *Facade* prelude.

**Verification**: `cargo test --workspace`, `cargo clippy -- -D warnings`, server + two clients smoke.

---

## Slice 3 — Extract `server` (transport, persistence, authoritative systems)

**Value**: enables the lean production server binary and clears gameplay logic out of the shared crate.

**Path**: move:
- `network/server.rs` → `crates/server/src/network/server.rs`
- `migrations/`, `plugins/persistence/*` → `crates/server/src/{migrations,persistence}/`
- `plugins/entity/systems.rs`, `plugins/entity/{player,enemy,boss}/systems.rs` (server-authoritative parts)
- `plugins/spells/` cast pipeline: `systems.rs`, `context.rs`, `aoe.rs`, `effects.rs` (server side)
- `stats/{plugin,systems}.rs`
- `crowd_control/{mod,systems,events}.rs`
- `plugins/player_movement.rs` authoritative simulation part

The game crate now depends on `bevymmo_server` (feature `server`).

**Acceptance criteria**:

- [ ] `bevymmo_server` compiles with `lightyear/server`, `sea-orm`, `tokio`, `uuid` — no `client` feature, no windowing.
- [ ] `cargo build --release --no-default-features --features server,netcode,udp,replication` compiles **without** `bevymmo_presentation`/`bevymmo_editor` in its tree (verify with `cargo tree`).
- [ ] Player spawn on connect, enemy AI, death/respawn, spells, CC, persistence all behave identically (smoke test two clients).
- [ ] Nothing in `server` imports from `client` or `presentation`.
- [ ] `AppMode::Server` run conditions in `server` use `bevymmo_shared::network::mode`.

**RED**: `cargo tree` test in CI that the prod build graph excludes presentation/editor.

**GREEN**: move server files; register a `ServerPlugins` facade re-exported as `bevymmo_server::ServerPlugins` so `main.rs` stays one line.

**Pattern**: *Facade* plugin group.

**Verification**: prod build + `cargo tree`, smoke test, `cargo test`.

---

## Slice 4 — Extract `client` (transport, input, targeting)

**Value**: client-only systems (input, prediction senders, targeting) become independent of rendering, and `presentation` can depend on `client` cleanly.

**Path**: move `network/client.rs`, `key_mapping.rs`, `targeting/` (systems), client-side `player_movement.rs` input, `CurrentTarget` resource stays in `shared` (used by UI).

**Acceptance criteria**:

- [ ] `bevymmo_client` compiles with `lightyear/client` + `shared`; no server features, no rendering.
- [ ] `MessageSender<SpellCastCommand>` wiring, cast-on-key, move-target selection, targeting systems behave identically.
- [ ] `bevymmo_client` never imports `bevymmo_presentation` or `bevymmo_server`.
- [ ] Host-client mode still composes both `bevymmo_server` and `bevymmo_client` in `main.rs`.

**RED**: none new (covered by slice-0 smoke + `cargo test`).

**GREEN**: move files; re-export `ClientPlugins`.

**Pattern**: *Facade* plugin group.

**Verification**: `cargo test`, client smoke with a running server.

---

## Slice 5 — Extract `presentation` (renderer, scenes, game UI)

**Value**: rendering and game UI become optional and never compile into the production server.

**Path**: move `renderer.rs`, `scenes/*`, all of `ui/*`, `game_state.rs` screens part, spell HUD (`cast_bar.rs`, cooldown UI), enemy/boss client visuals (`debug.rs`, `arena_visual.rs`, `dragon_visual.rs`, `ui/boss_bar/`).

**Acceptance criteria**:

- [ ] `bevymmo_presentation` compiles only with the `client` feature of bevy; it never imports `bevymmo_server`.
- [ ] All Bevy UI (`main_menu`, `spellbook`, `entity_bar`, `target_frame`, `cast_bar`, `settings`, `pause_menu`, `scoreboard`, ...) behaves identically.
- [ ] The headless prod server build excludes `bevymmo_presentation` (verify `cargo tree`).
- [ ] `scenes::base::follow_controlled_player`, renderer attach/detach on `GameScreen` transitions, all unchanged.

**RED**: extend the `cargo tree` CI check to fail if `presentation` appears in prod graph.

**GREEN**: move files; `PresentationPlugin` facade.

**Pattern**: *Facade* plugin group; *Adapter* (renderer already adapts replicated state → local components).

**Verification**: visual smoke test (client + host-client), `cargo test`.

---

## Slice 6 — Extract `editor` skeleton + `bins/game` move

**Value**: completes the target layout; the root package becomes a thin CLI binary; the editor crate exists (empty logic, wired for `cargo run -- editor`).

**Path**:

1. Move current root package into `bins/game/` (`Cargo.toml` + `src/main.rs`), update workspace members to `["crates/*", "bins/game"]`.
2. Add `AppMode::Editor` variant to the CLI (`main.rs`), gated behind feature `editor` (non-default).
3. Create `crates/editor` with a stub `EditorPlugin` that registers nothing yet (or a placeholder system logging "editor placeholder").
4. `bins/game/Cargo.toml` gains `editor = ["dep:bevymmo_editor"]`.

**Acceptance criteria**:

- [ ] `cargo run -- server|client|host-client` from `bins/game` behave identically.
- [ ] `cargo run -- editor` starts the app with `AppMode::Editor` and prints the placeholder without panic.
- [ ] Without the `editor` feature, `cargo run -- editor` fails to compile (CLI arm behind `#[cfg(feature = "editor")]`) — explicit, not silent.
- [ ] Prod server build still excludes editor + presentation.

**RED**: compile-time test that editor arm is feature-gated.

**GREEN**: move package, add CLI variant + stub crate.

**Pattern**: *Facade* + *Command* (CLI enum dispatch).

**Verification**: all four CLI modes smoke-tested; `cargo tree` checks; `cargo test`.

---

## Slice 7 — Cleanup, docs, CI

**Value**: the new structure is self-documenting and guarded.

**Path**:

- Remove any leftover `src/` at root; ensure no `crate::` reference crosses crate boundaries.
- Add `docs/crates.md` with: crate responsibilities, dependency graph (mermaid), "where does this type live" table, and "adding a new feature" recipe.
- Update `AGENTS.md` commands section: document `cargo run -- editor`, prod build, and the rule "shared = data only".
- Add CI job: `cargo check --workspace`, `cargo test --workspace`, `cargo clippy -- -D warnings`, and the `cargo tree` exclusion check for prod server.

**Acceptance criteria**:

- [ ] `cargo fmt --check`, `cargo check --workspace`, `cargo test --workspace`, `cargo clippy -- -D warnings` all green in CI.
- [ ] CI fails if the prod server graph contains `presentation` or `editor`.
- [ ] Docs updated; `docs/create-a-new-plugin.md` points to the crate of the new feature.

**Verification**: full CI + manual smoke of all modes.

---

## Suggested implementation order

1. Slice 0 (safety net)
2. Slice 1 (workspace skeleton)
3. Slice 2 (shared) — largest risk, do alone
4. Slice 3 (server)
5. Slice 4 (client)
6. Slice 5 (presentation)
7. Slice 6 (editor stub + bin move)
8. Slice 7 (cleanup/docs/CI)

Each slice is one PR, closed with `cargo fmt --check`, `cargo check --workspace`, `cargo test`, and the smoke checklist.

---

## Validation strategy

| Level | Command | When |
|---|---|---|
| Unit/integration | `cargo test --workspace` | every slice |
| Lint | `cargo clippy -- -D warnings` | every slice |
| Format | `cargo fmt --check` | every slice |
| Prod graph | `cargo tree -p game --no-default-features --features server,netcode,udp,replication` | slice 3+ (grep for presentation/editor) |
| Smoke | server + 2 clients: movement, scoreboard, death/respawn, spells | every slice |
| Editor smoke | `cargo run -- editor` (placeholder) | slice 6+ |

---

## Risks & mitigations

| Risk | Mitigation |
|---|---|
| Import rewrite bugs during extraction | Mechanical rewrite per file + compile after each file group; keep slices small |
| `shared` accidentally needing a server/client feature | CI compiles `shared` alone with data-only features |
| Circular dependencies | Rule D1 + `cargo check` catches; `cargo machete` to find unused deps |
| Behavior drift in spells pipeline (split across 3 crates) | Move the whole pipeline in one slice; smoke test spells explicitly |
| Long PR churn | One crate per slice; never mix extraction with feature work |

---

## Resolved decisions (confirmed by project owner)

| # | Question | Decision |
|---|---|---|
| 1 | Host-client role | **Dev/testing convenience only** — not a production target. Keep it working (cheap) but invest test/verification effort in pure `server` and pure `client` modes. Production = dedicated server + separate clients. |
| 3 | Editor binary | **`cargo run -- editor`** — mode in the fat game binary, feature-gated. Standalone `bins/editor` only if editor startup later diverges (different window config, asset defaults). |

## Open questions (still pending)

2. **`spawn_entity::<T>()` in shared vs server**: it is pure ECS composition with `Replicate`; keeping it in `shared` lets tests spawn entities anywhere. Acceptable? (Proposed: yes, shared.)
4. **Spell `cast()` implementations in shared**: they use `SpellCtx` which may need ECS commands/events. Confirm the `SpellCtx` contract stays transport-free (it should: it emits events consumed server-side).
5. **Settings**: move to `shared` (so editor reuses `Settings::load`) or keep in `bins/game`? Proposed: shared.
