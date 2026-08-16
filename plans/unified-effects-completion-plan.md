# Plan: Complete Unified Effects, Statuses, Crowd Control, Stats, and Runtime Verification

**Branch**: `feat/unified-effects-completion`
**Status**: Active
**Language**: English

## Goal

Finish the unified Effects/Statuses combat system so a player can apply, observe, cleanse, purge, and resolve statuses through the authoritative SpacetimeDB runtime, with no legacy scalar projectile/AoE or direct damage/healing effect paths remaining.

## Current Baseline

The repository already contains the following completed work and this plan must build on it rather than reintroducing parallel systems:

- `EffectSpec` is the shared effect vocabulary for `Damage`, `Heal`, `ApplyStatus`, `Cleanse`, and `Purge`.
- Declarative `#[status(...)]` content definitions generate `StatusDefinition` values.
- Existing content statuses include `Burn`, `Stun`, `Slow`, `Root`, and `Swift`.
- `ActiveStatus` and `ActiveStatuses` are replicated through SpacetimeDB/client bindings.
- Periodic effects, stat modifiers, stack scope, source ownership, expiry cleanup, and `RefreshPolicy` exist.
- `Cleanse` and `Purge` spells exist and are selectable through `PurityCharm`.
- Projectile and AoE rows use `Vec<EffectPayloadRow>` only.
- The following legacy paths have been removed:
  - `AoeEffect`;
  - `Projectile.damage`;
  - `AoeRegion.damage`;
  - `AoeRegion.healing`;
  - `pending_damage`;
  - `pending_healing`;
  - `emit_damage`;
  - `emit_heal`.
- `AoeTargetingRow` is persisted in the AoE schema.
- The local SpacetimeDB schema was reset, republished, and bindings regenerated.
- `cargo test --workspace`, workspace Clippy, and `spacetime build` currently pass.

This plan covers the remaining behavior and hardening work only.

## Non-Goals

- Reintroducing elemental damage types or elemental tags.
- Creating a second Buff/Debuff subsystem outside `StatusDefinition`.
- Preserving removed scalar projectile/AoE fields for compatibility.
- Replacing the authoritative SpacetimeDB simulation with client-side combat logic.
- Adding speculative stats or CC types without a concrete gameplay consumer.

## Acceptance Criteria

- [ ] A player can equip the purification item, use `Cleanse` and `Purge`, and observe the correct statuses disappear in a running client/server session.
- [ ] `Slow`, `Root`, and `Stun` have observable and tested movement/casting behavior through the status pipeline.
- [ ] Status reapplication obeys all supported refresh policies, including stack-dependent refresh behavior.
- [ ] Status expiry removes all owned periodic effects, modifiers, and control state without affecting another status or source.
- [ ] Crowd-control resistance, immunity, and diminishing returns are enforced authoritatively and deterministically.
- [ ] Effective stats are computed from base values plus item/status modifiers through one server-owned snapshot path.
- [ ] Damage, healing, mitigation, shields, penetration, and healing reduction use the documented deterministic order.
- [ ] Client status UI reflects replicated status state, category, stacks, source-independent duration, and removal without stale cards.
- [ ] Runtime integration tests cover the critical status/effect paths against a local SpacetimeDB instance.
- [ ] No effect-related legacy identifiers or scalar fallback branches remain in production code.
- [ ] All slices leave workspace tests, Clippy, module build, and relevant runtime checks passing.

## Working Rules

Every slice follows:

```text
RED → GREEN → MUTATE/KILL MUTANTS → REFACTOR → VALIDATE
```

Before writing production code for a slice:

1. Load the required implementation skills: `tdd`, `testing`, `mutation-testing`, and `refactoring`.
2. Present and confirm the slice acceptance criteria with the human.
3. Write a failing test that proves the observable behavior.
4. Implement the smallest production change.
5. Run mutation testing where the tooling supports the crate.
6. Strengthen tests for surviving meaningful mutants.
7. Refactor only if it improves clarity or removes duplication.
8. Run the slice validation commands.
9. Stop and request commit approval; never commit automatically.

Because `crates/stdb-module` is outside the Cargo workspace, module tests must use pure helper tests where possible and `spacetime build` plus local publish/runtime checks for WASM behavior.

## Slices

### Slice 1: Verify the published schema and purification gameplay path

**Value**: A player can use the already-built purification content against live replicated statuses instead of only passing unit tests.

**Actor**: Player.

**Trigger**: Equip `PurityCharm`, select `Cleanse` or `Purge`, and activate the ability in the running client.

**Observable outcome**: `Cleanse` removes cleanseable debuffs, `Purge` removes purgeable buffs, and the top-center UI removes the corresponding status card after replication.

**Production path**:

```text
Input/hotbar
→ ability selection
→ cast request
→ SpacetimeDB reducer
→ SpellCastContext
→ EffectSpec::Cleanse/Purge
→ status resolver
→ active_status deletion
→ client binding update
→ ActiveStatuses
→ status bar
```

**Files likely involved**:

- `crates/content/src/abilities/cleanse/`
- `crates/content/src/abilities/purge/`
- `crates/content/src/items/purity_charm/`
- `crates/stdb-module/src/reducers/spells.rs`
- `crates/stdb-module/src/sim/effects.rs`
- `crates/stdb-module/src/sim/status.rs`
- `crates/client/src/stdb/plugin.rs`
- `crates/presentation/src/ui/status_bar/`

**Acceptance criteria**:

- [ ] `PurityCharm` appears in the authoritative item registry.
- [ ] The player can select the three configured ability slots.
- [ ] Cleanse does not remove `Swift`.
- [ ] Purge does not remove `Burn` or `Stun`.
- [ ] Removed statuses disappear from `ActiveStatuses` and the UI.
- [ ] No client-only removal is performed before server replication.

**RED**: Add a local integration or reducer-level test that seeds one cleanseable debuff, one non-cleanseable debuff, one purgeable buff, and one non-purgeable buff, then resolves both effects and asserts the remaining rows.

**GREEN**: Use the existing `resolve_effect`/`cleanse`/`purge` pipeline; do not add a content-specific removal path.

**MUTATE**: Mutate filter selection, `cleanseable`/`purgeable` flags, and `max_statuses`; tests must fail for each incorrect mutation.

**KILL MUTANTS**: Add assertions for category isolation, non-dispellable status preservation, and removal ordering.

**REFACTOR**: Extract only reusable test fixtures for active status rows if the fixture is used by multiple later slices.

**Done when**: Runtime test or documented manual test passes against the freshly published local module, plus all normal gates pass.

---

### Slice 2: Make Slow and Root behavior observable

**Value**: A player can distinguish a speed reduction from a movement lock, and both behaviors are driven by status definitions rather than spell-specific code.

**Actor**: Player being affected by a status.

**Trigger**: An authoritative spell or test fixture applies `Slow` or `Root`.

**Observable outcome**:

- `Slow` reduces effective movement speed for its duration.
- `Root` prevents movement for its duration.
- Both statuses expire and restore the prior movement behavior.

**Production path**:

```text
ApplyStatus
→ StatusDefinition.control/stat_modifiers
→ ActiveStatus
→ effective stats / control state
→ movement validation
→ replicated status/UI
```

**Files likely involved**:

- `crates/content/src/statuses/slow.rs`
- `crates/content/src/statuses/root.rs`
- `crates/gameplay/src/crowd_control/`
- `crates/stdb-module/src/sim/status.rs`
- `crates/stdb-module/src/sim/crowd_control.rs`
- `crates/stdb-module/src/sim/movement.rs` or the authoritative movement reducer

**Acceptance criteria**:

- [ ] Slow changes the authoritative effective speed, not only the UI.
- [ ] Root blocks authoritative movement while active.
- [ ] Expiry restores movement without a manual client reset.
- [ ] Cleanse removes both statuses when configured as cleanseable.
- [ ] Applying a second Slow follows the declared stack/refresh policy.

**RED**: Add pure domain tests for speed calculation and server simulation tests for movement acceptance/rejection while Root is active.

**GREEN**: Route behavior through `ActiveStatus` and existing control/stat materialization; avoid a new `slow_entities` or `root_entities` table.

**MUTATE**: Mutate control kind, modifier sign, expiry comparison, and target filtering; tests must detect each error.

**KILL MUTANTS**: Add boundary tests at exactly zero remaining time, one tick before expiry, and one tick after expiry.

**REFACTOR**: Consolidate repeated control-state projection only if the same mapping is duplicated in more than one production path.

**Done when**: Slow and Root are behaviorally verifiable through the authoritative path and pass client/server regression checks.

---

### Slice 3: Complete status lifecycle and ownership guarantees

**Value**: Statuses can safely stack, refresh, expire, and clean up without leaking periodic effects, modifiers, or CC from another status/source.

**Actor**: Combat simulation.

**Trigger**: Apply, reapply, expire, cleanse, or remove a status from an entity.

**Observable outcome**: Only the intended status instance and its owned runtime children change.

**Production path**:

```text
ApplyStatus
→ stack/source lookup
→ refresh policy
→ ActiveStatus update
→ owned periodic/modifier/control materialization
→ expiry/removal cleanup
```

**Files likely involved**:

- `crates/stdb-module/src/sim/status.rs`
- `crates/stdb-module/src/sim/crowd_control.rs`
- `crates/stdb-module/src/tick.rs`
- `crates/gameplay/src/effects/status.rs`

**Acceptance criteria**:

- [ ] `StackScope::Global` merges the intended sources.
- [ ] `StackScope::PerSource` keeps independent source instances.
- [ ] `AddStacks` clamps at `max_stacks`.
- [ ] `RefreshPolicy::None` preserves remaining time.
- [ ] `RefreshPolicy::RefreshAll` resets to the new duration.
- [ ] `RefreshPolicy::RefreshNewStackOnly` resets only when a new stack is gained.
- [ ] `RefreshPolicy::Extend` adds time without corrupting the UI ratio.
- [ ] Expiry removes only children matching `origin_status_instance_id`.
- [ ] Removing one source's status cannot delete another source's control/modifier.

**RED**: Add pure helper tests for refresh/stack decisions and server-focused tests for child ownership and expiry cleanup.

**GREEN**: Keep ownership keyed by status instance ID; do not infer ownership from status ID or source alone.

**MUTATE**: Mutate max-stack clamping, source matching, instance matching, expiry comparison, and refresh branch selection.

**KILL MUTANTS**: Add two-source/two-instance fixtures and same-status/different-source cases.

**REFACTOR**: Extract a small lifecycle decision helper only if it makes all policy branches independently testable.

**Done when**: Lifecycle tests cover every policy and ownership invariant, with no leaked runtime child rows.

---

### Slice 4: Add Silence and authoritative cast interruption

**Value**: A player affected by Silence cannot start or continue spell casting, and removal immediately restores casting.

**Actor**: Player attempting to cast.

**Trigger**: Apply or expire a `Silence` status while the player is idle or casting.

**Observable outcome**: New cast requests are rejected with a deterministic reason; active casts are interrupted according to the declared policy; expiry/cleanse restores casting.

**Production path**:

```text
ApplyStatus(Silence)
→ ActiveStatus/control projection
→ cast request validation / cast advancement
→ CastBlockedReason or CastEnded
→ client cast UI
```

**Files likely involved**:

- `crates/content/src/statuses/silence.rs`
- `crates/gameplay/src/effects/status.rs`
- `crates/stdb-module/src/reducers/spells.rs`
- `crates/stdb-module/src/sim/spells.rs`
- `crates/stdb-module/src/sim/crowd_control.rs`

**Acceptance criteria**:

- [ ] Silence is declared as a cleanseable debuff.
- [ ] A silenced entity cannot start a spell cast.
- [ ] A cast interrupted by Silence produces the expected replicated cast-ended state.
- [ ] Silence does not block movement unless another status does so.
- [ ] Cleanse removes Silence and casting becomes available.

**RED**: Add reducer tests for cast validation and advancement tests for a Silence landing mid-cast.

**GREEN**: Reuse the existing cast interruption reasons and status control projection.

**MUTATE**: Mutate silence gating, interruption timing, and cleanup; tests must distinguish idle rejection from mid-cast interruption.

**KILL MUTANTS**: Add tests for instant, cast-time, and channeling spell kinds.

**REFACTOR**: Centralize control capability checks if the reducer and tick currently duplicate them.

**Done when**: Silence is an observable end-to-end status with deterministic cast behavior.

---

### Slice 5: Implement Resolve, CC resistance, and immunity

**Value**: Repeated crowd control has bounded impact and bosses/players can be configured as resistant or immune without spell-specific exceptions.

**Actor**: Combat simulation and PvP player.

**Trigger**: Repeated applications of the same or different hard-control statuses within a resolve window.

**Observable outcome**: Effective duration is reduced by resolve tier, then control is blocked during immunity; the result is replicated and inspectable.

**Production path**:

```text
ApplyStatus(control)
→ immunity check
→ CC resistance
→ resolve tier/diminishing returns
→ ActiveStatus/control duration
→ ResolveState update
→ replication/UI
```

**Files likely involved**:

- `crates/gameplay/src/effects/status.rs`
- `crates/gameplay/src/stats/`
- `crates/stdb-module/src/sim/status.rs`
- `crates/stdb-module/src/sim/crowd_control.rs`
- `crates/stdb-module/src/tables.rs` if authoritative resolve state must persist
- client bindings if resolve state is exposed for debugging/UI

**Acceptance criteria**:

- [ ] CC resistance reduces incoming control duration by a documented formula.
- [ ] Resolve tiers increase only for qualifying control applications.
- [ ] Resolve decays after its configured window.
- [ ] Immunity blocks qualifying control without creating a misleading active status.
- [ ] Cleanse does not incorrectly reset resolve unless explicitly specified.
- [ ] Boss immunity can be configured without a hard-coded boss ID branch.

**RED**: Add pure formula tests for resistance and diminishing returns, then authoritative application tests for tier progression and immunity.

**GREEN**: Implement the smallest typed `ResolveState` and `EffectImmunity` path needed by one hard-control status (`Stun`).

**MUTATE**: Mutate tier thresholds, duration multiplier, expiry window, and immunity comparison.

**KILL MUTANTS**: Add exact-boundary tests and tests for unrelated debuffs not increasing resolve.

**REFACTOR**: Separate policy calculation from table mutation so formulas remain host/WASM testable.

**Done when**: Repeated Stun applications are bounded and all acceptance criteria are covered by deterministic tests.

---

### Slice 6: Build one authoritative effective-stat snapshot

**Value**: Damage, movement, and UI consumers read the same effective stats after equipment and status changes.

**Actor**: Combat simulation and player stats UI.

**Trigger**: Equip/unequip an item or apply/remove a stat-modifying status such as `Swift` or `Slow`.

**Observable outcome**: One recomputed snapshot changes; all consumers observe the same value; removal restores the previous value.

**Production path**:

```text
Base stats + item modifiers + active status modifiers
→ dirty marker
→ effective snapshot recomputation
→ movement/combat/UI consumers
```

**Files likely involved**:

- `crates/gameplay/src/stats/components.rs`
- `crates/gameplay/src/stats/events.rs`
- `crates/gameplay/src/stats/formulas.rs`
- `crates/stdb-module/src/sim/status.rs`
- `crates/stdb-module/src/sim/combat.rs`
- `crates/client/src/stdb/plugin.rs`
- `crates/presentation/src/ui/player_stats/`

**Acceptance criteria**:

- [ ] Base, item, and status contributions are distinguishable in tests.
- [ ] Flat additions, percentage additions, multiplication, and override have a deterministic order.
- [ ] A status modifier is applied once, not once per tick.
- [ ] Removing a status restores the exact prior effective value.
- [ ] Movement and combat read the same snapshot.
- [ ] A dirty entity is recomputed once per simulation boundary.

**RED**: Add snapshot tests for `Swift`, `Slow`, one armor modifier, and removal/expiry.

**GREEN**: Implement only the fields required by current consumers, then extend the snapshot incrementally.

**MUTATE**: Mutate modifier order, sign, duplicate materialization, dirty reset, and removal recomputation.

**KILL MUTANTS**: Add multiple modifiers with opposing values and a same-tick apply/remove case.

**REFACTOR**: Remove duplicate stat calculations only after consumers use the snapshot.

**Done when**: Effective stats have one authoritative calculation path and the current status/item content uses it.

---

### Slice 7: Add deterministic damage, mitigation, shields, and healing rules

**Value**: Combat results are predictable and consistent for direct, periodic, projectile, and AoE effects.

**Actor**: Combat simulation.

**Trigger**: Resolve a damage or healing effect against an entity with defense, penetration, shield, or reduction modifiers.

**Observable outcome**: The same input snapshot always produces the same damage/health result and combat event.

**Production path**:

```text
EffectSpec::Damage/Heal
→ effective stats
→ defense reduction/penetration
→ mitigation
→ shield/healing reduction
→ health mutation
→ DamageEventRow
```

**Acceptance criteria**:

- [ ] Armor and resistance use documented formulas.
- [ ] Reduction is evaluated before per-hit penetration according to the chosen order.
- [ ] Shields absorb damage before health.
- [ ] Healing reduction affects healing but not damage.
- [ ] Periodic effects use the same instant resolver as direct effects.
- [ ] Zero/negative/NaN values are handled safely.
- [ ] Combat event rows preserve source and target attribution.

**RED**: Add formula tests and server resolver tests for one representative physical damage, shielded damage, reduced healing, and periodic tick.

**GREEN**: Keep damage/heal as instant `EffectSpec` resolution; do not add special periodic combat branches.

**MUTATE**: Mutate operation ordering, clamps, shield consumption, and source attribution.

**KILL MUTANTS**: Add order-sensitive tests such as armor reduction followed by damage and shield followed by health.

**REFACTOR**: Consolidate only duplicated numerical safety helpers.

**Done when**: Combat formulas and effect resolution are deterministic, tested, and used by all current effect sources.

---

### Slice 8: Complete runtime integration coverage and debugging observability

**Value**: Regressions in the authoritative effect pipeline are caught before a client release and are diagnosable in a live local session.

**Actor**: Developer/operator.

**Trigger**: Run the local integration suite or inspect a combat entity during a test session.

**Observable outcome**: The suite verifies status/effect state across server, schema, bindings, and UI-facing snapshots; debug output identifies source/status/instance ownership.

**Production path**:

```text
Local SpacetimeDB
→ published WASM module
→ reducer/tick execution
→ replicated rows
→ client conversion
→ presentation state
```

**Acceptance criteria**:

- [ ] Local setup instructions work from a clean SpacetimeDB volume.
- [ ] Schema reset/publish/generate is documented and repeatable.
- [ ] Integration coverage includes ApplyStatus, periodic tick, expiry, Cleanse, Purge, projectile, and AoE.
- [ ] Same-tick ordering is asserted for Apply→Cleanse→Apply and ArmorReduction→Damage.
- [ ] Active status snapshots are checked for status ID, source, stacks, potency, and remaining duration.
- [ ] Debug output can identify status instance ID and owned child rows without relying on client guesses.
- [ ] The test suite fails if any removed scalar legacy field returns.

**RED**: Add integration fixtures and a schema-shape assertion that searches generated row types for the new payload/targeting fields and rejects legacy fields.

**GREEN**: Reuse existing local scripts and binding conversion tests; do not add a second database harness unless required.

**MUTATE**: Mutate row ordering, missing replication updates, status source, and effect payload selection.

**KILL MUTANTS**: Add assertions for stale UI cleanup, missing definitions, unknown status IDs, and destroyed source entities.

**REFACTOR**: Extract shared integration setup only after at least two tests use it.

**Done when**: A clean local reset/publish/generate plus integration suite provides a repeatable green checkpoint.

---

### Slice 9: Finish status presentation with real assets and tooltips

**Value**: Players can understand active buffs/debuffs without relying on internal status IDs or symbol fallbacks.

**Actor**: Player.

**Trigger**: Hover/focus an active status card in the top-center bar.

**Observable outcome**: The player sees an icon, display name, category, stacks, remaining duration, source/description where available, and a readable tooltip.

**Production path**:

```text
StatusDefinition.presentation
→ replicated ActiveStatuses
→ status bar card
→ asset lookup + tooltip
```

**Acceptance criteria**:

- [ ] `Burn`, `Stun`, `Slow`, `Root`, and `Swift` have real assets or an explicit stable fallback.
- [ ] Missing assets never panic or break the status bar.
- [ ] Tooltip content comes from registered status definitions, not duplicated UI strings.
- [ ] Buff, debuff, and hard-control presentation are visually distinguishable.
- [ ] The duration bar and text remain synchronized after refresh and expiry.
- [ ] Keyboard/focus behavior is usable if the UI framework exposes focusable cards.

**RED**: Add presentation tests for asset path resolution, missing-asset fallback, category styling, tooltip text, and duration ratio.

**GREEN**: Add only the asset lookup and tooltip data required by current statuses.

**MUTATE**: Mutate icon IDs, missing registry definitions, category mapping, and duration updates.

**KILL MUTANTS**: Add unknown-status snapshots and zero/expired duration cases.

**REFACTOR**: Consolidate card construction only if tooltip and non-tooltip states otherwise duplicate layout logic.

**Done when**: The UI is understandable using production status metadata and all presentation tests pass.

## Cross-Slice Quality Gates

Before completing each slice:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd crates/stdb-module && spacetime build
```

When schema changes:

```bash
./scripts/stdb.sh reset
./scripts/stdb.sh publish
./scripts/stdb.sh generate
```

The reset is destructive to the local database and must be explicitly called out before execution.

Before every PR/slice completion:

1. Run mutation testing and record the report.
2. Run the refactoring assessment.
3. Verify no effect-related legacy identifiers remain:

```bash
grep -R "AoeEffect\|Projectile.*damage\|AoeRegion.*damage\|AoeRegion.*healing\|pending_damage\|pending_healing\|emit_damage\|emit_heal" crates
```

4. Preserve unrelated working-tree changes.
5. Do not commit without explicit user approval.

## Dependency and Ordering Notes

- Slice 1 should be completed before adding more combat rules because it validates the actual published schema and replication path.
- Slice 2 depends on the existing status registry but should be completed before Resolve so CC behavior has a concrete consumer.
- Slice 3 is a prerequisite for reliable Resolve and runtime integration tests.
- Slice 4 provides the second concrete CC behavior needed to validate generalized control handling.
- Slice 5 depends on Slice 4 and should start with Stun only; generalization can follow after the first behavior is correct.
- Slice 6 must precede broad combat-stat expansion so new stats have one calculation path.
- Slice 7 depends on Slice 6 because mitigation and healing rules consume effective stats.
- Slice 8 should be incrementally expanded after each previous slice, not postponed as a single final testing phase.
- Slice 9 is presentation polish and can run after Slice 1, but should consume the final status metadata from earlier slices.

## Definition of Done

The overall plan is complete when:

- Every acceptance criterion is checked.
- The local module can be reset, published, and regenerated from a clean database volume.
- All workspace tests and Clippy pass.
- `spacetime build` passes without warnings introduced by the feature.
- Runtime integration verifies status application, lifecycle, cleanup, cleanse, purge, periodic effects, projectile, and AoE.
- Advanced CC and effective-stat behavior are authoritative and deterministic.
- The legacy effect transport scan is empty.
- The final mutation-testing reports have been reviewed.
- The user has reviewed the completed slices and explicitly approved any commit.

---

## Open Questions

- What exact CC resistance and Resolve duration/multiplier formula should the game use?
- Should `Cleanse` reset Resolve, or should Resolve remain an anti-chain-control state?
- Should `Purge` be allowed to remove all positive statuses or only statuses marked `purgeable` with a tier limit?
- Which status categories should be visible to enemies in PvP, if any?
- Which final icon asset format should the project standardize on: PNG, WebP, or another Bevy-supported format?

These decisions should be resolved before implementing the affected slice; do not silently choose values that change gameplay balance.

---

*Delete this plan file when all slices are complete and the user has approved the final result.*
