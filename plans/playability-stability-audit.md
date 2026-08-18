# Plan: Playability and Stability Audit Fixes

**Branch**: `feat/playability-stability-audit`
**Status**: Active

## Goal

A player can log in, move, cast the starter staff, fight a boss that actually uses its rotation, and recover from a dropped SpacetimeDB socket — without HUD keys that never fire, self-cancelling charges, invisible offline bodies, or a frozen Connecting screen.

## Background

An 2026-08-19 audit of the live tree (not `plans/post-migration-remediation.md`) found the SpacetimeDB port itself is in good shape: terrain stepping is shared, tables are subscribed, most reducers use `*_then`, and render sync no longer judders the camera. What remains is a cluster of **playability** and **stability** holes. This plan sequences them as independently mergeable vertical slices.

Source map for implementers (do not treat older plans as current):

| Area | Live files |
|---|---|
| Combat keys | `crates/presentation/src/spells/input.rs`, `crates/presentation/src/spells/ui.rs`, `crates/client/src/stdb/combat_input.rs` |
| Starter kit | `crates/stdb-module/src/reducers/lifecycle.rs`, `crates/stdb-module/src/reducers/items.rs`, `crates/content/src/items/weapons/staff/mage_staff.rs` |
| Movement vs cast | `crates/stdb-module/src/reducers/movement.rs`, `crates/stdb-module/src/sim/spells.rs`, `crates/client/src/stdb/plugin.rs` |
| Boss / AI | `crates/stdb-module/src/sim/ai.rs`, `crates/content/src/spells/mod.rs` |
| Offline presence | `crates/stdb-module/src/reducers/lifecycle.rs`, `crates/stdb-module/src/sim/ai.rs` |
| Slow | `crates/content/src/statuses/slow.rs`, `crates/stdb-module/src/sim/combat.rs` |
| Visuals | `crates/presentation/src/renderer.rs`, `crates/client/src/stdb/plugin.rs` |
| Connection | `crates/client/src/stdb/plugin.rs`, `crates/presentation/src/ui/connecting.rs` |

`crates/stdb-module` is outside the Cargo workspace and is built with `spacetime build`, never `cargo build`. Extract pure helpers so domain rules can be tested with `cargo test` on `bevymmo_gameplay` / `bevymmo_content` / `bevymmo_client` without a WASM harness.

## Acceptance Criteria

- [ ] Pressing the keys drawn on the weapon HUD starts and **releases** a Charge the same way Q/W/E do.
- [ ] A newly created character can cast a staff ability without first opening inventory and the inscription panel.
- [ ] Holding right-click does not cancel a Charge; Stun/Root cannot be walked through by client `move_to` spam.
- [ ] Offline or character-select bodies are not valid AI or spell targets.
- [ ] Slow reduces movement speed; it does not reverse facing.
- [ ] The boss encounter fires registered rotation abilities (not silent `ai: no spell registered` logs).
- [ ] A failed or dropped SpacetimeDB connection leaves Connecting / InGame and shows a retryable error.
- [ ] Goblins, merchants, and the dragon use their authored GLBs; dummy/NPC placeholders stay cubes only when no asset exists.
- [ ] Inspecting an inventory item does not open Destroy; HUD clicks do not issue world move/target commands.
- [ ] Status icons do not despawn/respawn every simulation tick.
- [ ] Delayed cone abilities wait for their telegraph; leaving InGame despawns spell VFX.
- [ ] `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` stay green after each slice. Module changes additionally pass `cd crates/stdb-module && spacetime build`.

## Out of scope (named follow-ups, not this plan)

Whole-world subscription privacy / RLS, `with_confirmed_reads(false)`, shadow cascade distance, Swift unit scale, Burn stack potency, `CasterOnly` persistent regions, floating combat text, and deleting leftover Lightyear types. File a later plan if those become the next goal.

## Slices

Every slice follows RED-GREEN-MUTATE-KILL MUTANTS-REFACTOR. No production code without a failing test.

This repo is Rust/Bevy/SpacetimeDB. The JS `mutation-testing` skill does not apply. **MUTATE** here means: run the crate's `cargo test` for the touched package, then re-read the new tests against likely mutants (inverted `if`, wrong enum variant, `*= -0.5` vs `*= 0.5`, `just_pressed` vs `just_released`, missing `online` check). Strengthen any test that would still pass after those flips.

**Required implementation skills (every slice):** `tdd`, `testing`, `refactoring`, `rust-guidelines`. Load `Agents.md` before touching `crates/stdb-module`.

Present each slice's acceptance criteria and wait for confirmation before writing code. After the slice, present the work and wait for commit approval. One slice = one PR.

---

### Slice 1: HUD weapon keys use the same aim / charge / release path as Q/W/E

**Value**: A player who presses the keys printed on the hotbar (default 1/2/3) can charge and fire the starter staff instead of opening a Charge that never releases.
**Path**: Key press/release (`KeyAction::CastPrimary` / HUD slot) → single combat input helper → `eidolon_cast` on press for Charge, `release_cast` on release → `cast_state` row opens then completes → HUD cooldown starts on release.
**Intentionally skipped**: Armor D/R/F, remapping UX copy, deleting Lightyear leftovers.
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- The hotbar's `KeyAction` values are the same actions the cast system reads.
- A Charge ability bound to that action calls `eidolon_cast` on press and `release_cast` on release (not press-only).
- Q/W/E continue to aim Instant/CastTime and release Charge/Channel as they do today.
- `send_combat_inputs` no longer sends a second weapon `eidolon_cast` for the same slot on the same press.
**RED**: Extract (or add) a pure helper in `bevymmo_client` or `bevymmo_presentation` that, given `just_pressed` / `just_released`, `BlueprintExecution`, and `AbilityCastMode`, returns `{ start_cast, release_cast, open_aim }`. Fail today for `CastPrimary` + `Charge` because the current HUD path never requests release. Add a compile-time or unit assertion that `HOTBAR_SLOTS[i].action == SLOT_BINDINGS[i].0` after unification (today they differ: Digit1 vs KeyQ).
**GREEN**: Point HUD and keyboard at one binding table. Route Digit1/2/3 through `cast_abilities_on_key` (or extract that function and call it from both). Remove the weapon loop from `send_combat_inputs` (keep armor there until a later slice, or leave a stub).
**MUTATE**: Flip Charge to treat `just_pressed` as release — test must fail. Bind HUD to Q but send Digit1 — test must fail.
**KILL MUTANTS**: Cover Instant (aim on press, cast on release) so Charge logic is not accidentally applied to Arcane Wave if a non-Charge weapon is equipped.
**REFACTOR**: One `SLOT_BINDINGS` constant shared by HUD and input. No new abstraction beyond that.
**Done when**: Criteria met, `cargo test -p bevymmo_presentation -p bevymmo_client` pass, human approves commit.

---

### Slice 2: A new character can cast immediately after Play

**Value**: The first session is playable. The player is not staring at a lit hotbar that the server rejects.
**Path**: `join` reducer → grant `mage_staff` → equip into `EquipSlot::Weapon` → write a default `WeaponInscription` (`root_word = damage`) → `eidolon_cast` succeeds on the next Q/1.
**Intentionally skipped**: Auto-inscribing armor, changing known-language seed, hotbar string cleanup (slice 7 can touch hotbar if still stale).
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- After `join` for a new name, `equipment.weapon` is `Some(mage_staff)` and `root_inscription.root_word` is `Some("damage")`.
- `eidolon_cast` for Primary no longer returns `"no weapon equipped"` or `"weapon has no Root Word inscription"` on that fresh row.
- Existing characters are untouched (re-join of an owned name does not re-grant or overwrite inscriptions).
**RED**: Pure function `starter_equipment_for_new_character()` (or equivalent) returns weapon slot + inscription. Test that the instance id is set, item id is `mage_staff`, and root word is `damage`. A second test documents `grant_item` + empty equipment as the **old** shape and should fail once join writes equipment. If the module cannot host the test, put the builder in `bevymmo_gameplay` / `bevymmo_content` and call it from `join`.
**GREEN**: After `grant_item`, load inventory, move the granted instance into equipment, persist inscription. Recompute effective stats once. Keep `grant_item` as the only id-minting path.
**MUTATE**: Default root `"flame"` or leaving weapon empty must fail the test. Re-join overwrite of an existing inscription must fail a "do not touch existing characters" test.
**KILL MUTANTS**: Assert equipment slot count / `EQUIP_SLOTS` order still matches `rows::EQUIP_SLOTS`.
**REFACTOR**: Shared helper if `reducers/items.rs` already has equip internals; do not duplicate slot order a fourth time.
**Done when**: Criteria met, gameplay/content tests pass, `spacetime build` succeeds, human approves commit.

---

### Slice 3: Holding move cannot cancel a Charge or walk through Stun/Root

**Value**: Charging Arcane Bolt (or sitting in a stun) is not undone by a held right mouse button.
**Path**: RMB held → `send_move_commands` → `move_to` reducer → reject when cast is `CastTime` **or** `Charge`, or when `crowd_control` blocks movement → client stops sending / prediction follows cleared `move_target`.
**Intentionally skipped**: Changing InterruptOnMove channel policy (channel + move still cancels, which is intended). Render smoothing tweaks.
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- `move_to` returns an error (does not write `move_target`) when the caller has a `Charge` or `CastTime` row, or is Stunned/Rooted.
- The client does not call `move_to` while those states are observed locally (`ObservedCasts` / `CrowdControlState`).
- A Channel with `channel_movement_interrupts == true` still accepts `move_to` (move cancels the channel on the next tick, existing design).
**RED**: Pure `fn movement_intent_allowed(cast: Option<CastKind>, cc_blocks: bool) -> bool` in `bevymmo_gameplay::movement` (next to `should_block_movement_for_cast`). Today Charge is not blocked — test Charge + `cc_blocks = false` expecting `false` fails. Table-drive CastTime, Charge, Channel-interrupt, Instant, Stun.
**GREEN**: Call the helper from `reducers/movement.rs` and from `send_move_commands`. Extend `should_block_movement_for_cast` or replace it so prediction and the reducer share one policy.
**MUTATE**: Treating Channel-interrupt as blocked (or Charge as allowed) must fail a row in the table.
**KILL MUTANTS**: Dead + `move_to` still errors with the existing dead-character message (do not swallow it into the new branch).
**REFACTOR**: Delete unused `move_towards_target` in `client/src/movement.rs` only if the slice already touches that file and tests do not depend on it; otherwise leave it.
**Done when**: Criteria met, `cargo test -p bevymmo_gameplay -p bevymmo_client` pass, `spacetime build` succeeds, human approves commit.

---

### Slice 4: Offline characters are not combat targets

**Value**: Logging out or sitting on character select does not leave a punching bag for goblins, and other players cannot hit a body they cannot see.
**Path**: `leave` / `client_disconnected` / `expire_stale_presence` → character `online = false` → `living_players_near` and `potential_targets` skip that entity → AI leashes home, spells do not apply.
**Intentionally skipped**: Physically deleting the `game_entity` row (bodies can stay for a later "logout in place" feature). Client visual despawn already works.
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- `living_players_near` and spell `potential_targets` omit players whose `player.online` is false (or who have no `player` row).
- `expire_stale_presence` also clears `move_target` and cancels `cast_state` (same stop as `leave`).
- Online living players in range are still returned.
**RED**: Pure `fn is_combat_target(kind, state, online: Option<bool>) -> bool`. Cases: Player+Idle+online, Player+Idle+offline, Player+Dead+online, Enemy+Idle. Offline player must be `false` (fails if the helper only checks `state != Dead`).
**GREEN**: Resolve `owner_character_id` → `player.online` at the two call sites, or cache a `Vec` of online player ids once per tick. Presence expiry reuses `leave`'s stop path.
**MUTATE**: `online == false` treated as targetable, or enemies filtered out, must fail.
**KILL MUTANTS**: NPCs / dummies without a player row stay non-targets for AI player queries (they were never in `living_players_near`).
**REFACTOR**: One `online_player_ids(ctx)` snapshot used by AI and spells in the same tick if both would otherwise scan `player`.
**Done when**: Criteria met, tests pass, `spacetime build` succeeds, human approves commit.

---

### Slice 5: Slow halves speed instead of reversing it

**Value**: Getting hit by Arcane Wave makes the character sluggish, not moonwalk.
**Path**: Slow status → `stat_modifier` Multiply on Speed → `apply_stat_op` → `game_entity.speed` derived in `recalculate_effective_stats` → `step_on_terrain` uses a positive reduced rate.
**Intentionally skipped**: Swift magnitude, a global `1.0 + value` convention for every multiply (only Slow is wrong in content today).
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- `Slow` definition multiplies Speed by `0.5` (not `-0.5`).
- A character with base `movement_speed` S and only Slow equipped walks at `0.5 * S` (then `* 60` into `game_entity.speed`, same as today).
- Existing Slow unit test in `content/src/statuses/slow.rs` is updated and green.
**RED**: Change the assertion in `slow_is_a_cleanseable_speed_reduction_debuff_with_control` to `value == 0.5`. It fails on the current `-0.5`. Add a gameplay test: `apply_stat_op` Multiply `0.5` on Speed `0.15` yields `0.075`.
**GREEN**: Edit the `#[status(...)]` value. Do not change `apply_stat_op` unless a second test proves the fold itself is wrong.
**MUTATE**: Leaving `-0.5` or using `Add -0.5` must fail.
**KILL MUTANTS**: Refresh/stack metadata of Slow unchanged (duration 3, cleanseable, control = Slow).
**REFACTOR**: None unless the macro forces a comment that the multiplier is a factor, not a signed delta.
**Done when**: Criteria met, `cargo test -p bevymmo_content -p bevymmo_gameplay` pass, human approves commit.

---

### Slice 6: The boss can fire its rotation

**Value**: Crossing the arena ring starts a real fight, not a silent chase with `ai: no spell registered` every tick.
**Path**: `step_boss` → `run_rotation` → `request_cast` → `spells().get(id)` hits a registered `Spell` → Instant fires / CastTime writes `cast_state` → damage or VFX rows appear.
**Intentionally skipped**: Rebalancing numbers, new dragon VFX art, threat UI. One thin but complete kit is enough if every rotation id resolves.
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- `default_spells()` contains every id in `GROUND_ROTATION`, `AERIAL_ROTATION`, and `BERSERK_ROTATION` (`searing_breath`, `cinder_storm`, `wing_buffet`, `tail_sweep`, `dragon_claw`, `molten_eruption`, `cataclysm` as a **spell** id, not the hammer ability).
- `request_cast` no longer no-ops those ids (unit test: registry lookup succeeds; optional: Instant spell returns a cooldown).
- Enemy Fireball remains registered (goblin auto-attack still compiles).
**RED**: Test `default_spells()` contains each rotation id. Fails today (only `fireball`). Test that player hammer `cataclysm` (`BaseAbility`) and boss spell `cataclysm` do not collide if they share a string — if they do, the boss spell id must be renamed in the rotation **and** this test names the chosen id.
**GREEN**: Port or stub the missing `Spell` impls under `crates/content/src/spells/` with the old Bevy numbers if they still exist in git history; otherwise register Instant melee/cone/circle payloads that match the targeting enum (MainThreat, CasterCentered, DensestCluster). `cataclysm` as a boss spell must be a distinct `SpellId` if the hammer ability already owns `"cataclysm"` in a shared namespace (`cast_state.spell_id`).
**MUTATE**: Omitting one rotation id must fail the registry test. A channel with no duration must not be registered (AI skips those).
**KILL MUTANTS**: `priority_list_for(Dormant)` stays empty. Fireball still present.
**REFACTOR**: One `register_boss_spells(&mut SpellRegistry)` so `default_spells` stays readable.
**Done when**: Criteria met, `cargo test -p bevymmo_content` pass, `spacetime build` succeeds, human approves commit.

---

### Slice 7: Connection failure is visible and retryable

**Value**: Play / login never dead-ends on "Connecting…" when SpacetimeDB is down, and a mid-fight drop is not a silent freeze.
**Path**: Startup `connect()` or `frame_tick` / `on_disconnect` → `ConnectionFailure` + `GameScreen::MainMenu` (or a dedicated disconnected overlay) → player can retry, which calls `connect()` again and re-subscribes.
**Intentionally skipped**: Token refresh UX polish, exponential backoff tuning, interest-management subscriptions.
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- If `StdbConnection` is missing when Play is pressed, screen is **not** left on `Connecting`; `ConnectionFailure` is `Some`.
- If `frame_tick` fails or `conn.is_active()` becomes false while `InGame`, the player is taken out of gameplay and sees the failure string.
- Retry inserts a new `StdbConnection` (or documents that the process must be relaunched if the SDK forbids it) and does not keep simulating reducers against a dead socket.
**RED**: Pure `fn next_screen_after_connection_loss(current: Screen) -> (Screen, bool /*show_failure*/)` — `InGame`/`Connecting`/`Paused` → `MainMenu` + failure. Test `join_on_request` path: missing resource is a first-class state, not a silent skip (extract the decision from the system).
**GREEN**: `connect()` on failure writes `ConnectionFailure`. `pump_connection` watches `is_active()`. Play/auth systems check `resource_exists` and write failure if not. Add a retry button handler that calls `connect` + inserts the resource.
**MUTATE**: Staying on `InGame` after loss must fail. Treating a never-connected menu as a disconnect loop must fail (do not flash the overlay on first boot before the player hits Play).
**KILL MUTANTS**: Graceful `Shutdown` still exits via `finish_shutdown` and does not show "disconnected" as an error.
**REFACTOR**: One `ConnectionHealth` resource if `ConnectionFailure` is not enough; do not add a state machine beyond what the menu already has.
**Done when**: Criteria met, client tests pass, human approves commit.

---

### Slice 8: Replicated creatures use their authored models

**Value**: A goblin looks like a goblin, a merchant like a merchant, the dragon like the dragon — not every hostile as a tiny Vermithrax, every NPC as a 2 m cube.
**Path**: `game_entity.kind` (+ existing `Boss` marker) → `spawn_entity_meshes` chooses `PlayerAssets` / `BossDragonAssets` / new `GoblinAssets` / `MerchantAssets` / color cube fallback → `RenderedEntity`.
**Intentionally skipped**: Per-placeable `AssetHint` replication (needs a schema column). Animation graphs. Scale tuning beyond matching current player/boss constants.
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- `EntityKindRow::Boss` (client `Boss` component) uses `boss_dragon.glb`.
- `EntityKindRow::Enemy` uses `models/creatures/goblin.glb` (or a shared `CreatureAssets` handle).
- `EntityKindRow::Npc` uses `models/npcs/merchant.glb`.
- `EntityKindRow::Dummy` stays on the shared fallback cube.
- Client mapping may keep `Enemy | Boss => Hostile` for **gameplay** UI (nameplate color) but the renderer must not key the mesh off that collapsed enum alone.
**RED**: Pure `fn visual_prefab(kind: EntityKindRow, is_boss: bool) -> VisualPrefab` with an exhaustive match test. Today Hostile+not-boss would still return Dragon — that case must return Goblin and fail until the renderer is updated.
**GREEN**: Load goblin/merchant in `PresentationCorePlugin` asset collections. Branch `spawn_entity_meshes` on `Boss` vs `EntityKind` vs Neutral/Friendly. Keep the existing retry-until-assets-load loop.
**MUTATE**: Enemy → Dragon or Boss → Goblin must fail the prefab table.
**KILL MUTANTS**: Projectiles still use `RendererAssets.projectile_mesh`. Missing assets still retry (do not insert `RenderedEntity` without a handle).
**REFACTOR**: Fill `EntityVisualsPlugin` instead of growing `renderer.rs` if the match is cleaner there; one owner only.
**Done when**: Criteria met, `cargo test -p bevymmo_presentation` pass, human approves commit.

---

### Slice 9: Inventory inspect does not destroy, and HUD clicks do not move/target

**Value**: Opening an item tooltip or clicking a button cannot delete the item or send the character walking under the cursor.
**Path**: Pointer down on a slot → drag only after a pixel threshold → drop outside slots **cancels** unless the pointer is over an explicit destroy zone. World `select_move_target` / targeting / NPC pick run only when no UI `Interaction` is Hovered/Pressed.
**Intentionally skipped**: Redesigning the destroy confirmation modal, gamepad.
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- A click-release on a slot (movement &lt; threshold, e.g. 6 px) never enters the destroy-confirm path.
- A drag that starts on a slot and releases over empty world **cancels** the drag (item stays). Destroy requires the dedicated confirm control.
- While any gameplay HUD node is `Interaction::Pressed` or `Hovered` (inventory, chat, hotbar, inscription), RMB does not write `MoveTarget` and LMB does not change `CurrentTarget` / open the NPC sidebar.
**RED**: Pure `fn drag_outcome(start, end, over_slot, over_destroy_zone, threshold) -> DragOutcome` with ClickInspect / MoveItem / Cancel / RequestDestroy. Current "release off-grid ⇒ destroy" is the failing case. Pure `fn world_pointer_blocked(ui_pressed, ui_hovered) -> bool`.
**GREEN**: Inventory `drag.rs` uses the helper. Movement/targeting/NPC systems early-out on the block flag (query `Interaction` or a small `PointerOnHud` resource updated once per frame).
**MUTATE**: Threshold 0 (every click is a drag) must fail. `Hovered` not blocking world LMB must fail if AC requires it.
**KILL MUTANTS**: Drag onto another slot still moves/equips. Chat input still does not steal the first click that focuses it (existing `defocus_chat_on_world_click` stays consistent).
**REFACTOR**: One `hud_wants_pointer` helper used by all three world-click systems.
**Done when**: Criteria met, presentation tests pass, human approves commit.

---

### Slice 10: Status icons do not rebuild every tick

**Value**: A buff or Slow on the local player does not hitch the HUD twenty times a second.
**Path**: `active_status` row ticks `remaining_seconds` → client `status_signature_for` **ignores** remaining time → `ActiveStatuses` is not `Changed` on a pure countdown → status bar updates width/text on existing cards (or locally interpolates).
**Intentionally skipped**: Redesign of card layout, CC bar Root/Silence (called out as follow-up).
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- Two snapshots that differ only in `remaining_seconds` produce the same signature / do not mark `ActiveStatuses` changed after first apply.
- Adding/removing a status or changing stacks still updates the bar.
- Remaining time displayed on a card still counts down (local tick or a targeted node write, not a full children despawn).
**RED**: Unit test `status_signature_for` (move it to a `pub(crate)` fn if needed): same ids/stacks, different remaining → equal signatures. Today they differ because `remaining_seconds.to_bits()` is in the key — that is the failing assertion you want.
**GREEN**: Signature = `(id, stacks)` (and status_id if ids can be reused). Status bar: if set of instance ids unchanged, write duration bar width only; else rebuild.
**MUTATE**: Including remaining bits again must fail. Ignoring stack changes must fail.
**KILL MUTANTS**: Empty set still hides the root. First insert still spawns cards.
**REFACTOR**: None.
**Done when**: Criteria met, `cargo test -p bevymmo_client -p bevymmo_presentation` pass, human approves commit.

---

### Slice 11: Delayed cone abilities wait for their telegraph

**Value**: Arcane Wave's 0.15 s (and any other delayed cone) is a visible wind-up, not an instant hit on the fire tick.
**Path**: Ability `impact_delay` + cone geometry → `spawn_aoe_region` persists a row (not `apply_aoe_now`) → tick waits `pending_delay_seconds` → apply to entities still inside the cone, using stored aperture.
**Intentionally skipped**: Client subscription to `aoe_region` visuals (can be a tiny follow-up PR if VFX already fire from `spell_visual_effect`). Healing-circle `CasterOnly` seed bug.
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- `persistable_region` accepts `AoeShape::Cone { angle_deg }` when `duration_seconds > 0`.
- The row stores direction + aperture; the tick uses the same `AoeShape::contains` the domain already has.
- Circles with delay keep working. Instant cones (`duration == 0`) still resolve immediately.
**RED**: Table-drive `persistable_region` (extract to a pure fn taking the request, not `ReducerContext`): delayed cone → `Some`; zero-duration cone → `None`; delayed circle → `Some`. Current cone → `None` is the failing case.
**GREEN**: Add `angle_deg: f32` to `aoe_region` (schema change → `./scripts/stdb.sh reset` after publish). Persist cone; tick reconstructs `AoeShape::Cone`.
**MUTATE**: Persisting a cone with `angle_deg = 0` always, or applying before `pending_delay_seconds` elapses, must fail.
**KILL MUTANTS**: `ExcludeCaster` still seeds `affected` with the caster; do not change `CasterOnly` in this slice unless a test already covers it.
**REFACTOR**: Share contains-check between `apply_aoe_now` and the tick path.
**Done when**: Criteria met, gameplay tests pass, `spacetime build` succeeds, human approves commit. **Schema change: reset the local database.**

---

### Slice 12: Spell VFX do not survive leaving the world

**Value**: Returning to the menu does not leave glowing discs in an empty scene, and repeated casts do not leak unique meshes forever in one session.
**Path**: `dispatch_visual_effects` → spawn `SpellVisual` → leave `InGame`/`Paused` → despawn all `SpellVisual`. Optional: shared primitive handles for sphere/disc/box (not required to close the leak-on-leave).
**Intentionally skipped**: Full GPU instancing of every VFX; `update_colors` (next slice).
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- A system runs on `not_in_game` and despawns every `SpellVisual` (and click indicators if they have no owner).
- The `effects.rs` comment matches reality (cleanup lives in presentation, not "the binary").
**RED**: Bevy app test: spawn one `SpellVisual` while `Screen::InGame`, set `Screen::MainMenu`, run update, entity is gone. Fails until the system exists.
**GREEN**: Query + despawn next to `cleanup_entity_render`. Do not `assets.remove` unless you also own a pool (avoid yanking shared handles).
**MUTATE**: Cleanup that only removes `RenderedEntity` and leaves `SpellVisual` must fail.
**KILL MUTANTS**: InGame/Paused must **not** despawn live VFX mid-fight.
**REFACTOR**: Point the stale comment at the new system.
**Done when**: Criteria met, presentation tests pass, human approves commit.

---

### Slice 13: Shared entity materials are never mutated in place

**Value**: Recoloring one dummy or projectile cannot recolours every other entity that shares the cached handle; moving entities do not dirty a shared material at 20 Hz.
**Path**: `apply_entity` still writes `EntityColor` → `update_colors` **swaps** `MeshMaterial3d` to another cached handle (or no-ops if equal) instead of `Assets::get_mut`.
**Intentionally skipped**: Per-entity unique materials, GPU capture verification.
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- `update_colors` does not call `get_mut` on a handle stored in `RendererAssets`.
- Two entities sharing a color keep sharing one handle; changing one entity's `EntityColor` assigns a **different** handle to that entity only.
- Re-inserting the same `EntityColor` every row update does not dirty materials (compare before write, or stop re-inserting identical colors in `apply_entity`).
**RED**: Helper `fn material_after_color_change(shared_handle, old, new) -> HandleDecision` — SameColorNoop / SwapToCached. A test that a shared handle is not marked unique-mutated. Optional: `apply_entity` should not produce `Changed<EntityColor>` when the color bits are unchanged (test via a small function `color_insert_needed`).
**GREEN**: Implement swap-to-cache. In `apply_entity`, insert `EntityColor` only when it differs (or use `insert_if_neq` if available).
**MUTATE**: Always `get_mut` must fail. Skipping the swap when color actually changed must fail.
**KILL MUTANTS**: Projectiles without a color change still render. Missing cache still creates one material (do not panic).
**REFACTOR**: Reuse `RendererAssets::get_or_create_color_material`.
**Done when**: Criteria met, presentation tests pass, human approves commit.

---

### Slice 14: Aim and click use this frame's camera

**Value**: While walking, the aim preview, click ring, and ability ground point sit on the same pixel as the cursor, not last frame's camera.
**Path**: `RenderSync::Transforms` writes player `Transform` → `RenderSync::Camera` writes `GameCamera` → click/aim systems in `RenderSync::Project` (or after Camera) read `camera_view(&Transform)` like floating UI already does — not `GlobalTransform`.
**Intentionally skipped**: Raising the targeting sphere radius; NPC infinite-ray fix can ride along if it is a one-line change in the same helper, otherwise a follow-up.
**Required implementation skills**: `tdd`, `testing`, `refactoring`, `rust-guidelines`.
**Acceptance criteria**:
- `cursor_ground_point`, `cursor_ray` / `resolve_click_to_ground`, and aim preview use `GameCamera`'s **local** `Transform` via `camera_view`, not an arbitrary `Camera3d`'s `GlobalTransform`.
- Fallback Y=0 path returns `None` when `direction.y` is near zero (no NaN target).
**RED**: Test `resolve_click_to_ground` / ray-plane helper: parallel ray → `None` (today `send_move` fallback can divide by zero). Document in a unit test that the camera used is `GameCamera` (type-level: function signature takes the same view the UI uses).
**GREEN**: Change signatures to `(&Camera, &Transform)` + `camera_view`. Gate queries with `With<GameCamera>`. Fix the Y=0 fallback.
**MUTATE**: Using last-frame `GlobalTransform` is hard to unit-test; kill the NaN mutant and the "first Camera3d wins" mutant by requiring `GameCamera`.
**KILL MUTANTS**: No camera → `None`, no click sent. Surface miss → `None`, not a Y=0 point under the map.
**REFACTOR**: One `cursor_ray_this_frame` in `client/src/movement.rs` used by movement, targeting, spells, NPC.
**Done when**: Criteria met, client + presentation tests pass, human approves commit.

---

## Pre-PR Quality Gate

Before each PR:

1. `cargo test` for every touched workspace crate; `cargo test --workspace` if the slice crossed more than two crates.
2. `cargo clippy --workspace --all-targets -- -D warnings`.
3. If `crates/stdb-module` changed: `cd crates/stdb-module && spacetime build`. Schema changes require `./scripts/stdb.sh reset` locally and a note in the PR.
4. Mutation review: invert each new boolean and confirm a test fails (see slice MUTATE).
5. Refactoring assessment: no new unused abstractions; comments that would lie after the change are rewritten in the same PR.
6. Manual smoke (slices 1–3, 6–8 especially): `docker compose up -d spacetimedb`, publish if the module changed, `cargo run -- client`, one live click-cast-move pass.

## Suggested PR stack

```
Slice 1  HUD charge/release
Slice 2  Starter kit equipped + inscribed
Slice 3  move_to vs Charge/CC
Slice 4  Offline untargetable
Slice 5  Slow factor          ← independent; can land beside 1–4
Slice 6  Boss spells
Slice 7  Connection recovery
Slice 8  Creature GLBs        ← independent of 6
Slice 9  Pointer / inventory
Slice 10 Status bar signature
Slice 11 Cone telegraph       ← schema reset
Slice 12 VFX cleanup
Slice 13 Shared materials
Slice 14 This-frame aim
```

Slices 5, 8, 10, 12, 13 are independently mergeable and should not sit behind the combat stack if that stack stalls.

---

*Delete this file when the plan is complete. If `plans/` is empty, delete the directory.*
