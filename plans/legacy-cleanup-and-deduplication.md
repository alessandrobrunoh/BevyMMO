# Plan: Legacy Code, Dead Code, and Duplication Cleanup

**Branch**: TBD
**Status**: Draft — based on a full-workspace audit (5 parallel searches over all 13 crates, ~60k LOC), 2026-08-17

## Goal

Remove code that no longer executes, consolidate logic that has been copy-pasted across the workspace, and correct documentation/naming left behind by prior refactors — without changing observable game behavior. This is a cleanup plan, not a feature plan: every phase must leave `cargo build --workspace` and `cargo test --workspace` green, and must not touch gameplay logic beyond deleting unreachable code or merging identical implementations.

## Relationship to other plans

- **`post-migration-remediation.md`** already diagnosed the dead lightyear transport stack and several of the duplicated-logic pairs (movement stepper, modifier fold, death predicates) from the "why is the client silent" angle, and additionally lists concrete *bugs* (invisible entities, unwired UI actions, missing subscriptions) that are out of scope here. Where this plan's phase 1 overlaps with that plan's section 3.2/6, this plan is the actionable checklist; do not duplicate the bug-fix items (sections 1, 2, 5 of that plan) here.
- **`account-chat-admin.md` Slice 3** already owns wiring `Account.role`/`RoleRow::Admin` into a real permission policy and replacing `require_gm`'s `BEVYMMO_GM_IDENTITIES` allowlist. This plan's audit re-confirmed that gap (`crates/stdb-module/src/world.rs:458-491`, `crates/stdb-module/src/tables.rs:53-56`) but does **not** own fixing it — that work belongs to Slice 3, in progress alongside the account/session/character refactor. Do not implement that here; only flag it if it regresses further during phase 2/3 work in `stdb-module`.

## Ground truth and verification method

Every deletion or merge below was checked with a workspace-wide grep (not just crate-local) for the item's name before being listed. Line numbers are a snapshot from the 2026-08-17 audit and will drift — re-grep immediately before editing each item, don't trust the line number blindly.

Per-phase verification, run in this order after each phase:

1. `cargo build --workspace` — must stay warning-free (it already is; the workspace has no `#[allow(dead_code)]`/`#[allow(unused)]` escape hatches in the affected crates, so anything newly unreachable after a deletion will show up as an actual `dead_code` warning, not stay silently hidden).
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cd crates/stdb-module && spacetime build` (only for phases touching `stdb-module`)
5. For phase 1 (lightyear removal) and phase 3.1 (movement raycast merge): manual smoke test via the `run` skill — move the character, confirm click-to-move still lands where clicked.

If a listed item turns out to have a caller that the grep missed (macro-generated code, `dyn` dispatch, reflection), stop and drop that item from the phase rather than force the deletion.

---

## Phase 1: Delete the dead lightyear transport stack

This is the single largest, lowest-risk win: an entire parallel networking stack that was superseded by the SpacetimeDB client and is never instantiated by any `App` in the workspace. Confirmed by grepping `ClientTransportPlugins`, `ProtocolPlugin`, and each type below across the full workspace — the only matches are their own definitions and each other.

**Delete:**

- `crates/network/src/network/protocol.rs:176-302` — `ProtocolPlugin` and everything it exists to register:
  - `Channel1` (line 29), `Channel2` (line 32) — check `Channel2` isn't pulled in by something outside this dead path before deleting (last known use was `crates/client/src/network/client.rs:156`, itself being deleted in this phase).
  - `MoveCommand` (lines 54-57), `UpdateInscriptionRequest` (lines 154-160), `UpdateAbilitySelectionRequest` (lines 169-173).
- `crates/client/src/network/client.rs` (whole file, ~195 lines) — `ClientTransportPlugins` and its systems: `connect_on_intent`, `disconnect_on_intent`, `handle_connected`, `handle_disconnected`, `cleanup_disconnected_clients`, `handle_predicted_spawn`, `handle_controlled_spawn`, `handle_interpolated_spawn`, `receive_messages`, `receive_spell_visual_effects`, `lower_controlled_saturation`.
- `crates/client/src/network/runtime.rs` (whole file, ~105 lines).
- `crates/client/src/input/mod.rs` and `crates/client/src/input/key_mapping.rs` — pure re-export of `GameSettingsResource`/`KeyAction`, explicitly documented as superseded by `user_settings`; `crate::input` is not imported anywhere, including inside `client` itself.
- `crates/network/src/network/mode.rs:35-37` — `has_server()` run condition, never used as a condition anywhere; keep `has_client` (lines 31-33), which is live.
- `crates/network/src/network/mode.rs:9-15,22-24` — `AppMode::Server` and `AppMode::HostClient` variants, never constructed at runtime (`bins/game/src/main.rs:24-26,51` hardcodes `AppMode::Client`, and its own CLI doc says "There is no `server` or `host-client` any more"). Collapse `AppMode` to the single variant actually used, or keep the enum shape but delete the dead-construction paths — decide based on how much call-site `match` cleanup that forces; don't do a wider `AppMode` redesign in this phase.

**Fix up call sites left with permanently-empty state** (do not delete these systems, just note them so a reviewer isn't surprised the resource/component filters return nothing — the underlying bug of "these UI paths never receive server data" is `post-migration-remediation.md`'s problem, not this phase's):

- `crates/presentation/src/player_stats/systems.rs:46`, `crates/presentation/src/death_screen/systems.rs:98`, `crates/presentation/src/ui/debug_position.rs:56` — `Option<Res<ClientConnectionConfig>>`.
- `crates/presentation/src/spells/cast_bar.rs:117,150` — `With<ConnectedClient>`.

**Also delete (adjacent dead config/infra, confirmed orphaned once the above is gone):**

- `crates/app-support/src/settings.rs` — `Settings::database_url`, `ServerSettings`/`server.bind_addr`, `ClientSettings`/`server_addr`/`client_addr`. Only consumer was `crates/server`, already removed from the workspace (`exclude = [...]` in root `Cargo.toml` no longer lists it because the directory itself is gone — confirm with `git log -- crates/server` before deleting, in case it's mid-removal rather than fully gone). Do **not** touch `GatewaySettings` (lines 75-83) — real consumer in `apps/gateway/src/main.rs:48-51`.
- `crates/world/src/manifest.rs:582-589` — `MapManifest::is_v1()`/`is_v2()`, never called.
- `crates/domain/src/lib.rs:9` — `pub use bevymmo_gameplay::{ids, math};` re-export, never used via that path anywhere in the workspace.

**Acceptance criteria:**

- `cargo build --workspace` produces zero warnings (in particular, no new `dead_code` warnings appear from things that were only reachable through the deleted plugin).
- `bins/game` still builds and runs; click-to-move and the SpacetimeDB connection still work (manual smoke test).
- No remaining reference to `lightyear` in `bins/game`'s dependency tree that isn't already vestigial (note only, don't chase transitive `Cargo.lock` cleanup in this phase unless it's trivial).

---

## Phase 2: Remove unreferenced dead code (no structural change)

Smaller, independent items. Each was grepped across the full workspace; none are behind `#[allow(dead_code)]` because none of these crates use that attribute — they are simply unreachable `pub` items that the compiler doesn't flag. Group by crate so each can be its own small commit.

### `crates/presentation`

- `crates/presentation/src/entity/boss_arena_visual.rs`, `boss_dragon_visual.rs`, `enemy_debug.rs` — not declared in `entity/mod.rs`, so not compiled at all. Confirm with the maintainer whether these were mid-implementation (boss arena/dragon visuals, enemy debug overlay) before deleting outright — if the intent is still live, wire them into `entity/mod.rs` instead of deleting. Default action: delete, since `EntityVisualsPlugin::build` is an intentionally empty extension point per its own doc comment.
- `crates/presentation/src/ui/settings/panels/general.rs:86` (`refresh_general_panel`), `graphics.rs:136` (`refresh_graphics_panel`), `keybinds.rs:62` (`refresh_keybinds_panel`) — never registered as a system, never called; bodies are empty. `reset_keybinds_on_button` (`ui/settings/systems.rs:301`) mutates `KeyCapture` widgets directly instead of calling `refresh_keybinds_panel`, confirming the hook was never wired up.
- `crates/presentation/src/ui/target_frame/components.rs:11,15,19` — `TargetNameText`, `TargetHpText`, `TargetKindText` markers, never inserted or queried (contrast with `entity_bar/components.rs`, where the equivalent markers are real and documented as deliberately kept).
- `crates/presentation/src/ui/inventory/components.rs:15` — `InventoryWindow` marker, superseded by the generic `CardWindow` system (`ui/inventory/systems.rs:18,51,405`).

### `crates/gameplay`

- `crates/gameplay/src/items/definition.rs:134-138` — `weapon_abilities()`, marked `#[deprecated(note = "use Item::ability_loadout")]`, zero remaining callers. Delete now that the migration it was easing is complete.
- `crates/gameplay/src/entity/events.rs` — `DeathEvent`, `RespawnedEvent`; leftover ECS events from the pre-SpacetimeDB Bevy server, whose emitting functions (`mark_dead_entities`, `handle_respawn_request`, `enemy_respawn`) no longer exist anywhere (`crates/stdb-module/src/sim/ai.rs:12` explicitly notes the old function isn't there). Confirm nothing in `presentation` reads these events via `EventReader` before deleting (grep found none).
- `crates/gameplay/src/spells/events.rs:40` — `SpellCastRequest::caster_centered()`.
- `crates/gameplay/src/abilities/inscription.rs:68` — `SecondaryWord::with_intensity()`.
- `crates/gameplay/src/abilities/inscription.rs:153` — `AbilityInscription::from_slot()`.
- `crates/gameplay/src/spells/context.rs:454` — `SpellCastContext::emit_aoe_with_targeting()`.
- `crates/gameplay/src/spells/context.rs:482` — `SpellCastContext::emit_modifier()`.
- `crates/gameplay/src/spells/components.rs:125` — `SpellCooldowns::get_remaining()`. Note: this method's own doc comment admits it returns *elapsed*, not *remaining* time ("preserved as-is from the Bevy version") — delete rather than fix the name, since nothing depends on it either way.
- `crates/gameplay/src/stats/events.rs:57-61` — `HealEvent { target, source, amount }`, never constructed or read anywhere; healing goes through `ModifierEffect::HealOverTime` instead. Confirm this isn't a planned hook for a near-term feature (e.g. instant-heal spells) before deleting — check open plans for a healing feature first.

### `crates/client` (excluding `src/stdb/module_bindings/*`, which is generated)

- `crates/client/src/stdb/commands.rs:136-148` — `set_armor_inscription`.
- `crates/client/src/stdb/commands.rs:151-163` — `cast_spell` (legacy `SpellHotbar` path; player input now goes through the Eidolon/`AbilityId` pipeline per `crates/presentation/src/spells/input.rs:1-5`).
- `crates/client/src/stdb/commands.rs:223-226` — `stop`.
- `crates/client/src/movement.rs:150-172` — `is_valid_movement`.
- `crates/client/src/movement.rs:138-142,182-211` — `resolve_ground_position`, `step_towards_2d_target`; both live only in this file's own `#[cfg(test)]`, with production code going through `move_towards_target`/`step_on_terrain` (re-exported from `bevymmo_gameplay::movement`). Keep the tests only if they exercise logic still covered elsewhere; otherwise delete function and tests together.
- `crates/client/src/targeting/resources.rs:28,33,38` — `CurrentTarget::none()`, `is_some()`, `is_none()`.

### `crates/world`, `crates/network`

Already listed in phase 1 (`is_v1`/`is_v2`, `has_server`, `AppMode` dead variants) since they're part of the same lightyear-adjacent cleanup — don't split into a separate commit.

**Acceptance criteria per crate:** `cargo build -p <crate>` warning-free, `cargo test -p <crate>` green, no new dead-code warnings surfaced in downstream crates that depended on these items only for re-export purposes.

---

## Phase 3: Fix the misleading name/doc left by a prior refactor

Not dead code, but a landmine for the next person reading it:

- `crates/client/src/targeting/systems.rs:69-126` — function is named `select_target_with_right_click` and documented as "Target selection system with right click", but the body checks `mouse_buttons.just_pressed(MouseButton::Left)` (line 81). Rename the function (and doc comment) to match the actual binding; do not change the binding itself without confirming with the maintainer which button targeting is supposed to use today (it may have been intentionally moved to left-click when point-and-click movement took right-click).

**Acceptance criteria:** name and doc match behavior; no behavior change; `cargo test -p bevymmo_client` green.

---

## Phase 4: Consolidate duplicated logic

Each item is a genuine copy-paste, not superficial similarity. Do these as separate small refactor commits so each is easy to review and bisect if something regresses.

### 4.1 Movement raycast (medium risk — touches live input path)

`crates/client/src/player_movement.rs:39-96` (`select_move_target`) and `crates/client/src/stdb/plugin.rs:1551-1612` (`send_move_commands`) both independently: read `MouseButton::Right`, get the active `Camera3d`, call `viewport_to_world`, and resolve the ray to ground with the same Y=0 fallback. A third, partial copy of the "get ray from camera" step exists in `crates/client/src/targeting/systems.rs:85-96`.

- Extract a single `fn resolve_click_to_ground(...) -> Option<Vec3>` (or similar) covering camera-ray + ground resolution, reusing the existing `resolve_ray_to_ground` helper for the terrain part.
- `select_move_target` writes to `MoveTarget`, which today is read only to be cleared in `crates/presentation/src/spells/input.rs:242` — it does not currently drive movement. Decide explicitly whether `send_move_commands` should start reading `MoveTarget` (removing the duplicate raycast entirely) or whether `MoveTarget` should be deleted as unused state — don't leave both paths independently computing the same click. Default recommendation: make `send_move_commands` consume `MoveTarget` instead of recomputing it, since that's the smaller diff and keeps a single source of truth.
- Smoke test manually (harness `run` skill): click to move on flat ground and on a slope/step, confirm the destination is the same before and after.

### 4.2 UI helpers (low risk)

- `get_hp_fill_color` duplicated verbatim in `crates/presentation/src/ui/entity_bar/systems.rs:278-286` and `crates/presentation/src/ui/target_frame/systems.rs:244-252`. Move to a shared location (e.g. `ui::theme` or `ui::bar`) and have both call sites use it.
- `get_or_spawn_root` duplicated with the same "find singleton or spawn a full-screen absolute `Node`" structure in `crates/presentation/src/ui/entity_bar/systems.rs:39-58` (marker `FloatingUiRoot`) and `crates/presentation/src/ui/crowd_control_bar/systems.rs:156-172` (marker `CrowdControlBarRoot`). Generalize to `fn get_or_spawn_root<M: Component + Default>(...)`.

### 4.3 Registry boilerplate in `crates/gameplay` (low risk, largest LOC win — ~150+ lines)

Nine near-identical `HashMap<Id, Arc<dyn Trait>>` wrappers, each with `register`/`get`/`contains`/`len`/`is_empty`:

`ModifierRegistry` (`abilities/modifier.rs:110-129`), `EssenceRegistry` (`abilities/essence.rs:67-86`), `RootWordRegistry` (`abilities/root_word.rs:77-104`), `AncientWordRegistry` (`abilities/ancient_word.rs:125-143`), `BaseAbilityRegistry` (`abilities/base_ability.rs:448-467`), `WeaponFamilyRegistry` (`items/weapon_family.rs:46-68`), `SpellRegistry` (`spells/registry.rs:41-71`), `ItemRegistry` (`items/registry.rs:48-77`), `StatusRegistry` (`effects/status.rs:163-...`).

- Introduce a generic `Registry<K, V>` (candidate location: `crates/core` or a new `gameplay::registry` module) covering the common `register`/`get`/`contains`/`len`/`is_empty` surface.
- `ItemRegistry` and `SpellRegistry` additionally have a `sorted_*` method; `items/registry.rs`'s comment already references `SpellRegistry::sorted_spells` as the model to follow — fold that into the generic type too (e.g. `sorted_by_id` returning `Vec<(&K, &V)>` or similar) rather than leaving it duplicated a tenth time.
- This is a mechanical but wide-touching change (9 call sites). Do it as its own commit, and run the full `stdb-module`/`content` test suite afterward since both depend on these registries at content-load time.

### 4.4 Secondary Ancient Word application in `crates/gameplay/src/abilities/resolve.rs` (medium risk — combat logic)

`resolve_root_inscribed_slot` (lines 164-193, weapon path) and `resolve_armor_inscribed_ability` (lines 234-255, armor path) both: apply the root word, then loop over secondary words checking `knows_ancient_word` → `is_compatible_with` → `transform_blueprint`, erroring identically on unknown/incompatible words. The only real difference is the sort order of secondary words (by phase for weapons, by `word_id` for armor).

- Extract `fn apply_secondary_words(blueprint: &mut _, words: &[_], known: &_, ancient_words: &_) -> Result<(), CastBlockedReason>`, parameterized by the pre-sorted word list so the two call sites keep their different sort order.
- Also merge the near-identical wrappers `cast_armor_inscribed_ability` (lines 260-283) and `cast_root_inscribed_slot` (lines 284-322) — same `resolve_* → manifest_blueprint → Ok(())` shape; pass the resolve function as a parameter/closure.
- This touches spell-cast resolution directly: run the full combat/spell test suite and, if feasible, a manual cast of both a weapon ability and an armor ability with secondary words attached before merging.

### 4.5 `crates/world/src/collision.rs` (low risk, perf-adjacent)

- Lines 41-59 and 61-82 (`CollisionGrid::build`, props loop vs. blockers loop) recompute the same "AABB scaled by center" algorithm with different variable names. Extract `fn scaled_aabb(translation: Vec3, scale: Vec3, shape: &Shape) -> Obstacle` (or equivalent) and call it from both loops.
- `point_in_triangle_2d` (lines 363-384) and `barycentric_coords_2d` (lines 390-412) recompute the same `v0`/`v1`/`v2`/`dot00`/`dot01`/`dot02`/`dot11`/`dot12`/`inv_denom`/`u`/`v` quantities; `resolve_triangle_mesh` (lines 318, 347-348) calls both in sequence on the same triangle, doubling the work on every positive hit. Merge into one function returning both the containment test and the barycentric coordinates, and update `resolve_triangle_mesh` to call it once.
- These are hot-path collision functions — add/keep unit test coverage on both the merged AABB helper and the triangle test (points inside/outside/on edge) before and after, since this is exactly the kind of function where a sign or axis-order slip during merging is easy to introduce and easy to miss visually.

### 4.6 `crates/stdb-module/src/reducers/lifecycle.rs` (low risk)

- `select_character` (line 294) and `select_character_cleared` (line 417) are near-identical — merge into a single `fn set_active_character(ctx: &ReducerContext, character_id: Option<u64>)`.
- `claim_npc_item` (`reducers/items.rs:254-255`) calls both `caller_character(ctx)?` and `caller_entity(ctx)?`, and `caller_entity` (`lifecycle.rs:451-457`) internally calls `caller_character` again — the same `ctx.sender() → Session → Player` chain is resolved twice in one reducer. Derive the entity id from the already-resolved `character` instead of calling `caller_entity` a second time.
- `account.rs:186` and `lifecycle.rs:160` both do `ctx.db.session().identity().delete(&ctx.sender())` (in `logout` and `client_disconnected` respectively) — low priority, only extract a shared helper if this cleanup work is already touching both files for another reason; not worth a standalone commit on its own.

**Acceptance criteria (phase 4, all subsections):** `cargo test --workspace` green after each subsection; `spacetime build` green after 4.4 and 4.6 (both touch `stdb-module`-adjacent or `stdb-module`-internal code); no change in resolved combat/movement outcomes observable in manual smoke testing.

---

## Explicitly out of scope

- **`crates/stdb-module/src/world.rs:458-491` (`require_gm`) and `RoleRow::Admin` wiring** — owned by `account-chat-admin.md` Slice 3, in progress.
- **Legacy `Inscription`/`WeaponInscriptions` vs. `RootWord`-based model** (`crates/gameplay/src/abilities/inscription.rs`) — both models are live in production; consolidating them is a design decision (which model wins), not a mechanical cleanup, and needs its own plan.
- **Dual spell path (Eidolon/`AbilityId` for players, legacy `SpellId`/`SpellHotbar` for NPCs/bosses)** — intentional per existing doc comments in `spells/input.rs`, `spells/mod.rs`, `spells/available_choices.rs`, `spells/cast_bar.rs`; only worth revisiting if/when NPCs and bosses are migrated to the Eidolon pipeline, which is a separate feature decision.
- **Everything in `post-migration-remediation.md` sections 1, 2, 5** (invisible entities, unwired client→server actions, missing subscriptions, SpacetimeDB security/perf items) — those are behavior bugs and architecture gaps, not dead code or duplication, even though some share root causes with phase 1 of this plan.

---

## Suggested commit order

1. Phase 1 (lightyear stack removal) — biggest LOC win, lowest behavioral risk since none of it executes today.
2. Phase 2, crate by crate (`presentation`, `gameplay`, `client`) — independent, small, easy to review.
3. Phase 3 (naming fix) — trivial, can ride along with phase 2's `client` commit.
4. Phase 4.2, 4.3, 4.5 (UI helpers, registry generic, collision helpers) — mechanical consolidations, no behavior change intended.
5. Phase 4.1, 4.4, 4.6 last — these touch live input/combat/session logic and benefit from the smaller, already-landed cleanups reducing noise in the diff.

## Pre-PR quality gate (each commit)

1. `cargo build --workspace` — zero warnings.
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cd crates/stdb-module && spacetime build` (phases/commits touching `stdb-module`)
5. `git diff` contains only the intended deletion/merge — no incidental formatting-only churn in unrelated files.
6. Manual smoke test via the `run` skill for phase 1 and phase 4.1 specifically (movement/click-to-move).
