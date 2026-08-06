# Plan: Crowd Control Framework + Stun Field Spell (orange AoE)

## Goal

Introduce a **first-class Crowd Control (CC) system** and the first spell that
uses it: **Stun Field** — an orange, ground-targeted AoE that, `0.5s` after
being cast, **stuns every entity inside its radius** for a configured duration.

While stunned:
- The entity **cannot move** (point-and-click movement frozen, both server and
  predicted client).
- The entity **cannot cast** (cast requests rejected server-side).
- A **`CrowdControlBar` UI** (orange) appears above the entity's head and drains
  as the stun expires. When the stun ends, the entity can act again.

The plan deliberately builds a **generic CC framework** (not a stun-specific
hack) so that future CC types (Root, Silence, Slow, Fear…) reuse the same
component, replication, UI, and gating logic.

---

## Current Architecture (relevant recap)

| Concern | Where | Why it matters here |
|---|---|---|
| Spell trait + registry | `src/plugins/spells/context.rs`, `registry.rs` | New spell implements `Spell`. |
| Generic AoE lifecycle | `src/plugins/spells/aoe.rs` | Reads `AoeEffect` payload, agnostic of spell. We add a new variant. |
| AoE payload enum | `context.rs::AoeEffect` | `Damage`, `Heal`, `ApplyModifier`. Add `CrowdControl`. |
| Movement gating | `src/plugins/player_movement.rs` (`should_block_movement_for_cast`) | Extend with CC check. |
| Cast request handling | `src/plugins/spells/systems.rs` | Add CC rejection at validation time. |
| Stat modifiers | `src/stats/{events.rs,modifiers.rs}` | **Numeric** buffs/debuffs only. CC is NOT a stat — kept separate. |
| Screen-space floating bars | `src/plugins/spells/cast_bar.rs`, `src/ui/entity_bar/` | Pattern to mirror for the CC bar. |
| Replicated components | `src/network/protocol.rs` (`app.component::<T>().replicate()`) | New `CrowdControlState` registered here. |
| Existing spell example (delayed AoE) | `src/spells/meteorite/` | Stun Field follows the same shape. |
| Default spellbook | `src/plugins/spells/components.rs::DEFAULT_PLAYER_SPELL_IDS` | Add the new spell here. |
| Keybinds | `src/plugins/key_mapping.rs` | Bind a key (proposed: `R`). |

---

## Design Decisions

### D1. CC is a first-class concept, NOT a stat modifier

The existing `stats` modifier system models **numeric** changes (`Speed * 1.2`,
`Armor + 5`). A Stun is a **behavioral gate**: it suppresses movement and
casting, it has a UI affordance, and it composes with future CC types
(diminishing returns, immunity, cleanse). Modeling it as a `Speed = 0` modifier
would be a leaky hack and would not block casting.

**Decision**: Introduce a dedicated `crowd_control` plugin with its own
component, events, and lifecycle systems. Stats and CC remain decoupled.

### D2. Server-authoritative CC state, replicated + predicted

Following the pattern of `EntityState` / `VitalStats`, the authoritative CC
state lives on the server and is replicated to clients. We use
`.replicate().predict()` so the **predicted** local player immediately stops
moving the moment the server applies the stun, avoiding rubber-banding.

```rust
// network/protocol.rs
app.component::<CrowdControlState>()
    .replicate()
    .predict();
```

The duration tick is server-authoritative; clients only **read** the component
for the UI and for input gating.

### D3. CC payload on the generic AoE system

To stay consistent with `AoeEffect::Damage` / `Heal` / `ApplyModifier`, we add:

```rust
pub enum AoeEffect {
    Damage { ... },
    Heal { ... },
    ApplyModifier { ... },
    CrowdControl {
        kind: CrowdControlKind,
        duration_seconds: f32,
        once_per_entity: bool,
        targeting: AoeTargeting,
    },
}
```

The AoE system, on impact, writes an `ApplyCrowdControlEvent`. A CC-system
consumer mutates `CrowdControlState` on the target. This keeps the AoE system
spell-agnostic.

### D4. Multiple CC effects compose on one entity

`CrowdControlState` holds a `Vec<ActiveCrowdControl>`. A new stun **refreshes**
(replaces) an existing stun of the same kind to avoid stacking, and coexists
with other kinds (future Root/Silence). `has_blocking_cc()` returns true if any
effect that suppresses actions is active (Stun is blocking; a future Slow would
not be).

### D5. Movement gating extension (minimal, surgical)

Add a single helper `crate::plugins::crowd_control::is_blocked_by_cc(...)` and
call it alongside the existing `should_block_movement_for_cast` in:
- `server_move_to_target` (server-authoritative)
- `predict_move_to_target` (client prediction)

The guard-clause style already in place is preserved.

### D6. Cast gating at validation time

A stunned entity's cast requests must be rejected server-side **before** any
cooldown/cast-time is started. The check goes in the cast-validation system
(`plugins/spells/systems.rs`) so it covers all spells uniformly.

### D7. Stun Field is a one-shot burst AoE (not a persistent field)

Spec: "blocca tutte le persone al suo interno dopo 0.5 secondi". This is a
**snapshot** of who is inside at the 0.5s mark — exactly like Meteorite's
impact. The AoE region despawns right after applying the CC; the stun then
persists on each affected target for its own duration.

If a persistent stunning field is desired later, that's a separate spell using
the same `CrowdControl` payload with a longer-lived region.

### D8. Orange visual: warning circle + detonation flash

Client-only `visual.rs` mirroring Meteorite's structure:
- 0.5s orange semi-transparent warning circle on the ground.
- At t=0.5s, a brief expanding flash (no falling rock — Stun Field is ground
  magic).
- Color: `Color::srgb(1.0, 0.55, 0.0)` base, emissive orange.

### D9. CC bar UI reuses the `cast_bar.rs` screen-space pattern

A new `src/ui/crowd_control_bar/` module projects an orange draining bar above
each stunned entity. It is **independent** of the cast bar and stacks above it
(y-offset `3.1`, vs cast bar `2.55` and HP bar `2.0`).

---

## Data Model

### `src/plugins/crowd_control/components.rs`

```rust
/// Kinds of crowd control an entity can suffer.
///
/// Only `Stun` is implemented now; the enum is extensible so future CC types
/// (Root, Silence, Slow, Fear) share the same component and UI plumbing.
#[derive(
    Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect, Default,
)]
pub enum CrowdControlKind {
    #[default]
    Stun,
    // Future: Root, Silence, Slow, Fear, ...
}

/// Whether a CC kind suppresses actions (movement + casting) entirely.
///
/// Used by movement and cast gating. A future `Slow` would return `false`
/// here and be handled as a stat modifier instead.
impl CrowdControlKind {
    pub fn is_blocking(self) -> bool {
        matches!(self, CrowdControlKind::Stun)
    }
}

/// One active CC effect on an entity.
#[derive(Serialize, Deserialize, Clone, Debug, Reflect)]
pub struct ActiveCrowdControl {
    pub kind: CrowdControlKind,
    /// Remaining time before this effect expires (server-authoritative).
    pub remaining_seconds: f32,
    /// Original duration, used by the UI to render the bar fill.
    pub total_seconds: f32,
}

/// Server-authoritative CC state, replicated (and predicted) to clients.
///
/// Holds every active CC effect on the entity. Adding a new effect of the same
/// kind refreshes (replaces) the existing one; different kinds coexist.
#[derive(
    Component, Serialize, Deserialize, Clone, Debug, Default, Reflect,
)]
#[reflect(Component)]
pub struct CrowdControlState {
    pub effects: Vec<ActiveCrowdControl>,
}

impl CrowdControlState {
    pub fn is_empty(&self) -> bool { self.effects.is_empty() }

    /// True if any active effect blocks actions (movement + casting).
    pub fn has_blocking_cc(&self) -> bool {
        self.effects.iter().any(|e| e.kind.is_blocking())
    }

    /// Refreshes (or inserts) a CC effect of the given kind.
    pub fn apply(&mut self, kind: CrowdControlKind, duration_seconds: f32) {
        if let Some(active) = self.effects.iter_mut().find(|e| e.kind == kind) {
            active.remaining_seconds = duration_seconds;
            active.total_seconds = duration_seconds;
        } else {
            self.effects.push(ActiveCrowdControl {
                kind,
                remaining_seconds: duration_seconds,
                total_seconds: duration_seconds,
            });
        }
    }

    /// Advances all timers and drops expired effects.
    pub fn tick(&mut self, delta_seconds: f32) {
        for effect in &mut self.effects {
            effect.remaining_seconds = (effect.remaining_seconds - delta_seconds).max(0.0);
        }
        self.effects.retain(|e| e.remaining_seconds > 0.0);
    }

    /// Longest remaining blocking effect, for the UI to render.
    pub fn longest_blocking(&self) -> Option<&ActiveCrowdControl> {
        self.effects
            .iter()
            .filter(|e| e.kind.is_blocking())
            .max_by(|a, b| a.remaining_seconds.partial_cmp(&b.remaining_seconds).expect("finite"))
    }
}
```

### `src/plugins/crowd_control/events.rs`

```rust
/// Request to apply a CC effect to a target.
///
/// Emitted by the AoE system (and, in the future, by direct-hit spells, traps,
/// or boss mechanics) and consumed by the CC lifecycle system.
#[derive(Debug, Clone, PartialEq, Message)]
pub struct ApplyCrowdControlEvent {
    pub target: Entity,
    pub source: Option<Entity>,
    pub kind: CrowdControlKind,
    pub duration_seconds: f32,
}
```

---

## Implementation Phases

### Phase 1 — CC framework (`src/plugins/crowd_control/`)

New plugin, mirroring the layout of `src/plugins/spells/`.

```
src/plugins/crowd_control/
├── mod.rs          // CrowdControlPlugin, re-exports
├── components.rs   // CrowdControlKind, ActiveCrowdControl, CrowdControlState
├── events.rs       // ApplyCrowdControlEvent
└── systems.rs      // apply_cc_events, tick_crowd_control
```

**Systems:**
- `apply_cc_events` (server-only, `FixedUpdate`, `run_if(has_server)`): reads
  `ApplyCrowdControlEvent`, ensures the target has a `CrowdControlState`
  component (inserts if missing), calls `state.apply(kind, duration)`.
- `tick_crowd_control` (server-only, `FixedUpdate`, `run_if(has_server)`):
  `state.tick(delta)` on every entity with `CrowdControlState`. The component
  remains attached (empty) after expiry to avoid insert/remove churn; the UI
  hides when `is_empty()`.

**Plugin registration:** `src/plugins/mod.rs` → `app.add_plugins(crowd_control::CrowdControlPlugin);`

**Replication:** `src/network/protocol.rs`:
```rust
app.component::<CrowdControlState>().replicate().predict();
```

### Phase 2 — `AoeEffect::CrowdControl` variant

`src/plugins/spells/context.rs`:
- Add variant (see D3).
- `AoeEffect::targeting()` returns the inner `targeting`.

`src/plugins/spells/aoe.rs::apply_aoe_effect_to_targets`:
- New match arm writing `ApplyCrowdControlEvent` via a new
  `MessageWriter<ApplyCrowdControlEvent>` parameter (plumb through
  `update_aoe_regions`). Respect `once_per_entity` exactly like `Damage`.

### Phase 3 — Movement gating

`src/plugins/player_movement.rs`:
- Add `Option<&CrowdControlState>` to both `server_move_to_target` and
  `predict_move_to_target` queries.
- Guard clause at the top of each loop body:
  ```rust
  if cc.map(|c| c.has_blocking_cc()).unwrap_or(false) {
      *state = EntityState::Idle;
      continue;
  }
  ```
- Keep the existing `should_block_movement_for_cast` check **after** the CC
  check (CC wins: a stunned caster can't move even mid-cast-time).

### Phase 4 — Cast gating

`src/plugins/spells/systems.rs` (the system that consumes
`SpellCastRequest`):
- Add `Option<&CrowdControlState>` to the caster query.
- Reject the request early if `has_blocking_cc()`:
  ```rust
  if cc_state.map(|c| c.has_blocking_cc()).unwrap_or(false) {
      // drop the request silently, no cooldown consumed
      continue;
  }
  ```
- Also cancel any in-progress `CastProgress` when CC is applied (Phase 1
  `apply_cc_events` can remove the `CastProgress` component from a stunned
  caster, mirroring movement-interrupt semantics).

### Phase 5 — Stun Field spell (`src/spells/stun_field/`)

```
src/spells/stun_field/
├── mod.rs          // pub mod definition; #[cfg(client)] pub mod visual; re-export
├── definition.rs   // StunFieldSpell: impl Spell
└── visual.rs       // client-only orange warning + detonation flash
```

**`definition.rs` constants:**

| Constant | Value (proposed) | Note |
|---|---|---|
| `ID` | `"stun_field"` | |
| `DISPLAY_NAME` | `"Stun Field"` | |
| `COOLDOWN_SECONDS` | `15.0` | Tunable. |
| `CAST_RANGE` | `12.0` | Ground-target clamp. |
| `AREA_RADIUS` | `4.0` | |
| `IMPACT_DELAY_SECONDS` | `0.5` | The 0.5s window from the spec. |
| `STUN_DURATION_SECONDS` | `2.0` | How long targets are stunned. |
| `ORANGE` | `Color::srgb(1.0, 0.55, 0.0)` | Shared with visual + UI. |

**`Spell::config`:**
```rust
SpellConfig::ranged_aoe(Self::COOLDOWN_SECONDS, Self::CAST_RANGE, Self::AREA_RADIUS)
```
(Instant cast — the 0.5s is the AoE's own `initial_delay`, not a cast time, per
the spec: "dopo 0.5 secondi che é stata Castata".)

**`Spell::cast`:**
```rust
let center = ctx.target_position
    .map(|t| Self::clamp_target_to_range(ctx.caster_position, t))
    .unwrap_or(ctx.caster_position);

ctx.emit_aoe_with_delay(
    center,
    Self::AREA_RADIUS,
    Self::IMPACT_DELAY_SECONDS,        // region lives just long enough to detonate
    Self::IMPACT_DELAY_SECONDS,        // delay before effect applies
    Self::ID,
    AoeEffect::CrowdControl {
        kind: CrowdControlKind::Stun,
        duration_seconds: Self::STUN_DURATION_SECONDS,
        once_per_entity: true,
        targeting: AoeTargeting::ExcludeCaster,
    },
);

ctx.emit_visual(Self::ID, center, center);
```

(`clamp_target_to_range` copied from Meteorite — consider extracting to a shared
helper in Phase 7 if a third spell needs it.)

**`visual.rs`:** mirror `meteorite/visual.rs` minus the rock; orange warning
circle for 0.5s, then a short expanding flash. Register
`spawn`/`animate` in `effects.rs` match arm and the `client_effect_systems`
chain.

**Registration:**
- `src/spells/mod.rs`: `pub mod stun_field;`
- `DEFAULT_PLAYER_SPELL_IDS` in `components.rs`: add `"stun_field"`.
- `src/plugins/spells/effects.rs`: add match arm dispatching to
  `stun_field::visual::spawn`.
- `src/plugins/key_mapping.rs`: bind to **`R`** (proposed; Q/W/E/R MOBA layout
  — confirm with user).

### Phase 6 — `CrowdControlBar` UI (`src/ui/crowd_control_bar/`)

Screen-space, projected above the head, **drains** as the stun expires.

```
src/ui/crowd_control_bar/
├── mod.rs
├── components.rs   // CrowdControlBarRoot, ScreenCrowdControlBar, CrowdControlBarParts
└── systems.rs      // sync + position + content update + cleanup
```

**Pattern** (copied from `cast_bar.rs`, simplified — no network observation
needed because `CrowdControlState` is a replicated component):

- `sync_screen_cc_bars`: for each entity with a non-empty, blocking
  `CrowdControlState`, ensure a bar exists; despawn bars whose entity has no
  blocking CC.
- `update_screen_cc_bars`: project the entity's `Position` (offset `+3.1 Y`),
  set fill width = `remaining / total * 100%`, set label `Stun {x.x}s`.
- Orange fill color (`StunFieldSpell::ORANGE`), dark backdrop.
- Cleanup root when leaving gameplay (mirror `cleanup_screen_cast_bars`).

**Plugin:** register in `src/ui/plugin.rs` (or wherever `EntityBarPlugin` is
wired), gated by `in_gameplay`.

**Stacking offsets** (so HP bar / cast bar / CC bar don't overlap):

| Bar | World Y offset |
|---|---|
| Entity HP bar | `2.0` |
| Cast bar | `2.55` |
| **CC bar** | **`3.1`** |

### Phase 7 — Polish & shared helpers (optional, deferred)

- Extract `clamp_target_to_range` into `plugins/spells` once a 3rd spell uses it.
- Add a cleanse spell / CC immunity component when needed.
- Diminishing returns: track recent CC history per target (future).

---

## File-by-File Change Summary

| File | Change |
|---|---|
| `src/plugins/crowd_control/mod.rs` | **NEW** — `CrowdControlPlugin`. |
| `src/plugins/crowd_control/components.rs` | **NEW** — `CrowdControlKind`, `ActiveCrowdControl`, `CrowdControlState`. |
| `src/plugins/crowd_control/events.rs` | **NEW** — `ApplyCrowdControlEvent`. |
| `src/plugins/crowd_control/systems.rs` | **NEW** — `apply_cc_events`, `tick_crowd_control`. |
| `src/plugins/mod.rs` | Register `CrowdControlPlugin`. |
| `src/network/protocol.rs` | `app.component::<CrowdControlState>().replicate().predict();` |
| `src/plugins/spells/context.rs` | Add `AoeEffect::CrowdControl` variant + `targeting()` arm. |
| `src/plugins/spells/aoe.rs` | Plumb `MessageWriter<ApplyCrowdControlEvent>`; new match arm. |
| `src/plugins/player_movement.rs` | CC guard in server + predict move systems. |
| `src/plugins/spells/systems.rs` | Reject casts while `has_blocking_cc()`. |
| `src/spells/stun_field/mod.rs` | **NEW**. |
| `src/spells/stun_field/definition.rs` | **NEW** — `StunFieldSpell`. |
| `src/spells/stun_field/visual.rs` | **NEW** — orange warning + flash. |
| `src/spells/mod.rs` | `pub mod stun_field;` |
| `src/plugins/spells/components.rs` | Add `"stun_field"` to `DEFAULT_PLAYER_SPELL_IDS`. |
| `src/plugins/spells/effects.rs` | Dispatch match arm for `stun_field` visual. |
| `src/plugins/key_mapping.rs` | Bind `R` (proposed) to `stun_field`. |
| `src/ui/crowd_control_bar/mod.rs` | **NEW**. |
| `src/ui/crowd_control_bar/components.rs` | **NEW**. |
| `src/ui/crowd_control_bar/systems.rs` | **NEW** — screen-space bar. |
| `src/ui/plugin.rs` (or `mod.rs`) | Register `CrowdControlBarPlugin`. |

---

## Suggested Implementation Order

1. **Phase 1** — CC framework + replication (no gameplay effect yet; add a debug
   system to apply a stun on a key for testing).
2. **Phase 3 + 4** — Movement + cast gating (verify a manually-applied stun
   freezes the player).
3. **Phase 2** — `AoeEffect::CrowdControl` (verify AoE can stun via a debug spell).
4. **Phase 6** — CC bar UI (orange draining bar visible above stunned entities).
5. **Phase 5** — `StunFieldSpell` definition + visual (the real player-facing spell).
6. **Phase 7** — Polish.

This order ensures each layer is independently testable before the next is
built on top.

---

## Validation Strategy

- `cargo clippy -- -D warnings` — must pass (project rule).
- `cargo test` — add unit tests for `CrowdControlState` (`apply` refresh,
  `tick` expiry, `has_blocking_cc`, `longest_blocking`).
- Manual: run `cargo run -- host-client`, cast Stun Field on a dummy/enemy,
  verify:
  - Orange warning circle appears for 0.5s.
  - At detonation, entities inside freeze.
  - Orange bar appears above their heads and drains over `STUN_DURATION_SECONDS`.
  - Movement resumes exactly when the bar empties.
  - The stunned entity cannot cast during the window.
- Edge cases to verify: caster excluded (`ExcludeCaster`), re-cast during stun
  (refresh, not stack), leaving the AoE during the 0.5s window (no stun applied
  — it's a snapshot at detonation).

---

## Open Questions

1. **Keybind**: proposed `R`. Confirm or pick another.
2. **Stun duration**: proposed `2.0s`. Tunable, confirm.
3. **Should Stun also cancel an in-progress cast on the stunned target?**
   Proposed: **yes** (Phase 4 removes `CastProgress`). Confirm.
4. **Should the caster be immune to their own Stun Field?**
   Proposed: **yes** (`AoeTargeting::ExcludeCaster`). Confirm.
5. **Friendly fire?** Current `AoeTargeting` variants are `Everyone` /
   `CasterOnly` / `ExcludeCaster`. If allies should be immune, that requires a
   new targeting variant (team awareness) — out of scope for this plan unless
   requested.
