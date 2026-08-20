# Create a new plugin

The `game` binary is a Bevy client. The authoritative server is the SpacetimeDB module (`crates/stdb-module`), not a Bevy process. There is no `EntityPlugin`, no lightyear `Replicate` recipe, and no `GameEntityBundle`. Game entities appear because the client mirrors `game_entity` rows; the renderer then attaches meshes.

## TL;DR

| Kind | Crate | Register |
|---|---|---|
| Rule the server must run | `bevymmo_gameplay` / `bevymmo_content` / `bevymmo_world` (rules) + `crates/stdb-module` (tables, reducers, tick). `bevymmo_domain` re-exports the rules crates. | Not a Bevy plugin. Publish the module, then `./scripts/stdb.sh generate`. See `docs/architecture.md`. |
| Bevy input, targeting, row mirroring | `bevymmo_client` | `app.add_plugins(...)` in `bins/game/src/main.rs`, next to `PlayerMovementPlugin` / `TargetingPlugin` |
| Rendering, scenes, HUD, menus | `bevymmo_presentation` | Child of `UiPlugin` (`crates/presentation/src/ui/plugin.rs`) or of `PresentationPlugin` |
| Composition only (CLI, window, which plugins exist) | `bins/game` | `build_app` in `bins/game/src/main.rs` |

One feature, one plugin. Parent plugins compose children (`PresentationPlugin` → `UiPlugin` → `ChatPlugin`). Do not add a HUD widget in `main.rs`.

## Where does it go?

Ask: does the server need this rule? If yes, it is not a Bevy plugin. The module cannot link Bevy.

| If it is… | It goes in… | It is not… |
|---|---|---|
| Damage, movement, spell effects, item definitions | `bevymmo_gameplay` / `bevymmo_content`, then a table/reducer/`sim::` step in `crates/stdb-module` | A client system that “also does it locally” with a second copy of the rule |
| Click-to-move, targeting, SpacetimeDB connection, row-to-entity mirroring | `bevymmo_client` | Presentation, and not `bins/game` beyond the one `add_plugins` line |
| Meshes, cameras, HUD, menus, VFX | `bevymmo_presentation` | A reducer, and not a second spawn path for the same entity |
| Window title, asset-root check, which plugins the process loads | `bins/game/src/main.rs` | Deep domain logic |

`AppMode::has_client` is only for composition in `main.rs`. Client plugins are client-only: do **not** `.run_if(has_client)` on their systems.

## How to add a presentation UI plugin

Example: a new HUD widget. Follow `ChatPlugin`, `ConnectingPlugin`, `TargetFramePlugin`.

1. Add a module under `crates/presentation/src/ui/` (`foo_hud.rs`, or `foo_hud/{mod.rs,plugin.rs,systems.rs}` if it has more than one file).
2. Export `pub mod foo_hud;` from `crates/presentation/src/ui/mod.rs`.
3. Implement `Plugin`. Register messages/resources on **this** plugin (`add_message` / `init_resource`), not in `main.rs`. Duplicate `add_message` is fine if the widget must work in tests without `StdbPlugin`.
4. Register it on `UiPlugin` in `crates/presentation/src/ui/plugin.rs`: `app.add_plugins(foo_hud::FooHudPlugin);`. `PresentationPlugin` already includes `UiPlugin`.

```rust
use bevy::prelude::*;
use crate::game_state::Screen;

pub struct FooHudPlugin;

impl Plugin for FooHudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FooHudState>();
        app.add_systems(Startup, setup_foo_hud);
        app.add_systems(
            Update,
            (
                sync_foo_hud_visibility.run_if(state_changed::<Screen>),
                update_foo_hud.run_if(in_state(Screen::InGame)),
            ),
        );
    }
}
```

**Systems and `run_if`.** Prefer `in_state(Screen::InGame)` at registration. Shared conditions live in `bevymmo_client::app_state` (presentation re-exports them as `crate::game_state`):

| Condition | Meaning |
|---|---|
| `in_state(Screen::InGame)` / `in_gameplay` | In the world, including with the pause overlay open. Pause does not stop simulation or network. |
| `in_unpaused_gameplay` | In game and `PauseOverlay::Off`. `State<PauseOverlay>` is absent in menus, so this must not require that resource. |
| `not_in_gameplay` | Menus / connecting. Use for cleanup. |
| `not_typing` | No text field has focus. Required on any system that reads keybinds (`Escape`, cast keys, toggle inventory). |

Keybind systems need both screen and typing: `.run_if(in_state(Screen::InGame)).run_if(not_typing)`.

**Menu / overlay visibility.** Spawn the tree once at `Startup`. Toggle `Node.display`; do not despawn/respawn the screen. Gate the visibility system with `state_changed::<Screen>()` (and `resource_changed::<AuthState>()` etc.). Combining two change detectors in Bevy 0.19 needs `or_eager`:

```rust
sync_visibility
    .run_if(state_changed::<Screen>.or_eager(resource_changed::<AuthState>));
// pause overlay / chat:
// .run_if(state_changed::<Screen>.or_eager(state_changed::<PauseOverlay>))
```

Spawn `Display` must match the default state (`Screen::MainMenu` + logged out). First `Update` may skip a `state_changed` system because `Screen` was already initialized:

| Widget | Spawn `Display` |
|---|---|
| Login | `Flex` |
| Character select (`MainMenuUi`) | `None` |
| Chat | `None` |

**3D scene** is not a display toggle: `scenes/base` spawns on `OnEnter(Screen::InGame)` and despawns on `OnExit`. Pause does not despawn the scene.

**Screen-space HUD** (nameplates, cast bars) that projects a world point into the viewport must run `.in_set(RenderSync::Project)`. `RendererPlugin` chains `RenderSync { Transforms, Camera, Project }` so the projection sees this frame’s smoothed transform and camera, not last frame’s.

**`PointerOnHud`.** Only `PointerPlugin` initializes it. `PlayerMovementPlugin`, `TargetingPlugin`, and `UiPlugin` all `add_plugins(PointerPlugin)`; Bevy keeps one copy by type. If the widget is a HUD node that should block world clicks, it already participates: `UiPlugin` copies UI `Interaction` into `PointerOnHud` in `PreUpdate`. World-click systems early-out via `hud_wants_pointer`. Do not init `PointerOnHud` yourself.

## How to add a client plugin

Example: a new input helper. Follow `PlayerMovementPlugin` / `TargetingPlugin`.

1. Add a module under `crates/client/src/` and `pub mod` it from `crates/client/src/lib.rs`.
2. Implement `Plugin`. Init its resources there. If it handles world clicks, `app.add_plugins(crate::pointer::PointerPlugin)` and early-out when `hud_wants_pointer` is true. If it reads keybinds, `.run_if(crate::app_state::not_typing)`.
3. Register it in `bins/game/src/main.rs` inside the existing `has_client()` block, next to targeting and movement:

```rust
app.add_plugins(bevymmo_client::player_movement::PlayerMovementPlugin);
app.add_plugins(bevymmo_client::targeting::TargetingPlugin);
app.add_plugins(bevymmo_client::foo_input::FooInputPlugin);
```

Do not put input in presentation. Presentation may read `CurrentTarget` / `MoveTarget`; it does not own the click.

Reducer calls (move, cast, chat) stay in `bevymmo_client::stdb`. A new input plugin should write a resource or message the stdb plugin already drains, or call through `stdb::commands` — it should not open its own SpacetimeDB connection.

## How game entities appear

The server does not spawn Bevy entities. `crates/stdb-module` writes `game_entity` (and related) rows. The client subscribes; `StdbPlugin` (`crates/client/src/stdb/plugin.rs`) drains those rows onto Bevy entities in `apply_entity`: `GameEntity`, `NetworkEntityId`, `Position`, `PlayerName`, `EntityColor`, `EntityKind`, `StdbAuthoritative`. The local character also gets `LocalPlayer`.

Presentation then adds visuals by **polling**, not by observers:

- `RendererPlugin::spawn_entity_meshes` queries `Without<RenderedEntity>` and inserts `Mesh3d` / glTF scene roots once assets exist.
- Do **not** add an `Add<Position>` observer for that. glTF collections (`PlayerAssets`, `CreatureAssets`, …) may still be loading when the row arrives; the observer used to drop those entities. The poll loop is the retry.

HUD that attaches to entities (nameplates) uses the same polling idea (`spawn_ui_for_new_entities` + a marker), not `Add<VitalStats>`.

Players, enemies, bosses, and dummies are kinds on **one** table, not one Bevy plugin per kind. A new kind is a module table/enum change plus renderer/prefab mapping (`visual_prefab` in `renderer.rs`), not a new `FooPlugin` under a deleted `bins/game/src/plugins/entity/` tree.

Server-side schema, reducers, and the tick: `docs/architecture.md` and `docs/database.md`.

## Screen, messages, tests

**`Screen`** is a Bevy `States` enum: `MainMenu`, `Settings`, `Connecting`, `InGame`. Pause is `PauseOverlay` (`Off` / `On`), a `SubStates` of `InGame`. It does not pause `Time`, `FixedUpdate`, or the network.

Writes go through `NextState<Screen>` / `NextState<PauseOverlay>`. Never mutate `State` directly. `NextState` applies in `StateTransition`, which runs **before** `Update`. A test that sets state from a system in `Update` needs a second `app.update()` before `State<…>` changes.

`GameStatePlugin` (`crates/client/src/app_state.rs`) does `init_state::<Screen>()`, `add_sub_state::<PauseOverlay>()`, and the connection/auth/typing resources. `bins/game` adds it unconditionally. Presentation re-exports the same types from `bevymmo_presentation::game_state`.

**Messages and resources** are registered on the owning plugin. `StdbPlugin` `add_message`s `ChatLine`, `SpellVisualEffect`, `SpellCastProgress`, `SpellCastEnded`, `ServerNotice`, `SpellCooldownState`. Presentation plugins that consume those in isolation (spell HUD, notices, cast bar) register them again so a `MinimalPlugins` test does not need the whole client.

**Cross-plugin order** uses `SystemSet`, not “hope they run in add order”:

- `RenderSync::{Transforms, Camera, Project}` — renderer / camera follow / screen-space UI.
- `ClientSimulation::Predict` — stdb prediction; ability input runs `.before` it.

**Headless tests.** `MinimalPlugins` does not include `StatesPlugin`; `init_state` panics without it. Presentation tests that touch `Screen` should call `crate::game_state::init_screen_states(app)` (adds `StatesPlugin` if missing, then `init_state::<Screen>()` and `add_sub_state::<PauseOverlay>()`).
