# Plan: Cast Time, Channeling, Meteorite & Movement Buff

## Goal

Extend the server-authoritative spells framework with two new mechanics:

1. **Cast Time** — a spell with a fixed wind-up before its effect fires. The caster must wait (and can be interrupted). The client shows a progress bar with remaining seconds.
2. **Channeling** — a spell whose effect scales with how long the player keeps the key held down (or stays in channel state). The longer you channel, the stronger the result.

Then ship three new pieces of content:

- **Meteorite** — AoE damage spell, 1s cast time, after cast completes a meteorite falls on the targeted ground location after 2s, dealing 50 damage to all enemies (not the caster).
- **Fix Healing Circle** — currently heals everyone in the circle; it must heal **only the caster**.
- **Swift (key F, channeling)** — while the player holds F they channel and receive a **+20% movement speed buff**. Releasing F (or being interrupted) ends the buff. The spell is **not** interrupted by movement (you can channel while running).

## Current Architecture (relevant recap)

```
Client (cast_spells_on_key) ── SpellCastCommand ──▶ Server
                                                        │
                                              handle_spell_cast_commands
                                                        │
                                              SpellCastRequest (event)
                                                        │
                                              process_cast_requests
                                                  ├─ validate (spellbook, cooldown)
                                                  ├─ build SpellCastContext
                                                  ├─ spell.cast(&mut ctx)
                                                  └─ drain pending_damage / pending_healing
                                                      pending_projectiles → spawn HomingProjectile
                                                      pending_aoes        → spawn AoeRegion
                                                      pending_visuals     → SpellVisualEffect (replicated)
```

Key files:

- `src/plugins/spells/context.rs` — `Spell` trait, `SpellConfig`, `SpellCastContext`, `AoeEffect`, `AoeSpawnRequest`.
- `src/plugins/spells/components.rs` — `Spellbook`, `SpellCooldowns`.
- `src/plugins/spells/systems.rs` — `process_cast_requests`, `tick_spell_cooldowns`, `register_builtin_spells`.
- `src/plugins/spells/aoe.rs` — `AoeRegion` + `update_aoe_regions` (generic AoE payload dispatcher).
- `src/plugins/spells/ui.rs` — client HUD (cooldown text only).
- `src/network/protocol.rs` — `SpellCastCommand`, `SpellVisualEffect`.
- `src/network/client.rs::cast_spells_on_key` — fires `SpellCastCommand` on `keys.just_pressed(key)`.
- `src/network/server.rs::handle_spell_cast_commands` — translates to `SpellCastRequest`.
- `src/plugins/key_mapping.rs` — `KeyBindings`.
- `src/spells/*` — concrete spell implementations (definition + visual).

Two important facts that shape the plan:

- The current cast pipeline is **fire-and-forget**: one key press → one `SpellCastCommand` → one `spell.cast(ctx)` invocation. There is no notion of "ongoing cast state".
- AoE `ApplyModifier` payloads have no concept of `caster exclusion` or `caster-only`. The caster is stored on `AoeRegion.caster` but `update_aoe_regions` applies the payload to **every** entity in range, including the caster and friends. This is the root cause of the Healing Circle bug.

---

## Design Decisions

### D1. Server-authoritative cast/channel state

Cast Time and Channeling require **persistent per-caster state**. This is added server-side as a new component on the caster entity. The client only mirrors progress for UI and sends "start" / "stop" intents via the existing reliable command channel. The server is the single source of truth.

### D2. Two cast models, unified pipeline

| Model        | Trigger                                  | Server state                              | When `spell.cast(ctx)` runs              |
| ------------ | ---------------------------------------- | ----------------------------------------- | ---------------------------------------- |
| Instant      | `just_pressed` (current behaviour)       | none                                      | immediately                              |
| Cast Time    | `just_pressed` (start intent)            | `CastProgress` component on caster        | once, when timer completes               |
| Channeling   | `pressed` (held)                         | `CastProgress { kind: Channeling }` on caster | repeatedly (per tick) while held         |

Channeling also needs a "release" intent (`just_released`, or pressing the same key again) so the server can flush the final accumulated effect and despawn the cast state.

### D2b. Movement interruption policy

Two distinct rules:

- **Cast Time**: ALWAYS interrupted by movement. No exceptions. This is the canonical "wind-up" tradeoff.
- **Channeling**: per-spell, via a new enum on `SpellConfig`:

  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum ChannelMovementPolicy {
      /// Movement cancels the channel (default for offensive channeled spells).
      #[default]
      InterruptOnMove,
      /// Movement is allowed; only release / re-press / death ends the channel
      /// (e.g. Swift: you must be able to benefit from the speed buff while moving).
      AllowMovement,
  }
  ```

  `SpellConfig` gets a `channel_movement: ChannelMovementPolicy` field, ignored for Instant and CastTime spells.

### D2c. Re-press interruption

Pressing the **same** spell key again while a channel is active **interrupts** the channel (equivalent to release). This is a hard rule for all channeling spells, independent of `ChannelMovementPolicy`. The client sends `SpellIntent::Release` on the second press (handled locally by tracking active channel state — see Phase 3).

### D3. Cast progress as a first-class concept

A single component `CastProgress` covers both Cast Time and Channeling:

```rust
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastKind {
    CastTime,   // fires once when finished, then removed
    Channeling, // applies effect each tick while held, removed on release
}

#[derive(Component)]
pub struct CastProgress {
    pub spell_id: SpellId,
    pub kind: CastKind,
    pub elapsed_seconds: f32,
    /// For CastTime: time required before the spell fires.
    /// For Channeling: not strictly required (open-ended) but capped by config.
    pub required_seconds: f32,
    pub target_position: Option<Vec3>,
    pub target_entity: Option<Entity>,
}
```

Rules:

- A caster may have at most one active `CastProgress`. Starting a new cast while one is in progress **cancels** the previous one (server-side) and the client gets a `SpellCastEnded { completed: false }` message so its UI clears.
- **Movement**:
  - Cast Time spells: any movement > small epsilon cancels. Always.
  - Channeling spells: cancellation depends on `SpellConfig::channel_movement` (`InterruptOnMove` vs `AllowMovement`). Swift uses `AllowMovement`.
- **Re-pressing the same spell key** during a channel cancels the channel (D2c).
- **Pressing a different spell key** cancels the current cast/channel and starts the new one (server-side swap).
- **Death** always cancels.
- Cooldown starts when the cast **completes** (CastTime) or is **released/interrupted** (Channeling), not when the cast begins. Channeling spells typically have short or zero cooldown since the time-investment is the channel itself — Swift uses `0.0`.

### D4. New protocol messages

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SpellIntent {
    /// Begin any spell (instant, cast-time, or channeling).
    Start {
        spell_id: String,
        target_position: Option<Vec3>,
        target_id: Option<u64>,
    },
    /// Release a channeling spell, or cancel a cast-time spell, or interrupt
    /// a channel by re-pressing its key (the client translates re-press into Release).
    Release { spell_id: String },
}
```

Replaces the current single-fire `SpellCastCommand`. Instant spells still use `Start` (server fires immediately, no `CastProgress` spawned, no `Release` needed).

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, bevy::prelude::Message)]
pub struct SpellCastProgress {
    pub caster_network_id: u64,
    pub spell_id: String,
    pub kind: u8,           // 0 = CastTime, 1 = Channeling
    pub elapsed_seconds: f32,
    /// CastTime: required wind-up. Channeling: 0.0 (open-ended, bar fills differently).
    pub required_seconds: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, bevy::prelude::Message)]
pub struct SpellCastEnded {
    pub caster_network_id: u64,
    pub spell_id: String,
    /// true = completed normally, false = interrupted/cancelled
    pub completed: bool,
}
```

The client maps `caster_network_id` to the local entity (via the existing `NetworkEntityId` mapping) and renders a **world-space billboard bar above the caster's head** for ALL players — local and remote. The local player additionally sees a screen-space bar in the HUD for clarity. Both are driven purely by replicated state, so observers stay in sync.

---

## Implementation Phases

### Phase 1 — Spell config + context for delayed/cast-time effects

**File:** `src/plugins/spells/context.rs`

1. Extend `SpellConfig` with three fields:

   ```rust
   pub cast_time_seconds: f32,                  // 0.0 = instant
   pub is_channel: bool,                        // true = channeling
   pub channel_movement: ChannelMovementPolicy, // ignored unless is_channel
   ```

   Update all `SpellConfig` constructors (`melee_aoe`, `ranged_single_target`, `ranged_aoe`, `new`) to default these to `0.0` / `false` / `ChannelMovementPolicy::default()` so existing spells stay instant.

2. Add a `with_cast_time(seconds)` and `with_channel(movement_policy)` builder on `SpellConfig` so spell definitions stay readable.

3. Add `Spell::cast_kind(&self) -> CastKind` with a default impl derived from config:
   - `is_channel == true` → `CastKind::Channeling`
   - `cast_time_seconds > 0.0` → `CastKind::CastTime`
   - else → `CastKind::Instant`

3. Add a new `AoeEffect` variant for caster-only / ally-exclusion semantics:

   ```rust
   AoeEffect::ApplyModifier {
       effects: Vec<ModifierEffect>,
       duration_seconds: Option<f32>,
       kind: ModifierKind,
       once_per_entity: bool,
       targeting: AoeTargeting,   // NEW
   }

   pub enum AoeTargeting {
       Everyone,                  // current behaviour
       CasterOnly,                // only region.caster
       ExcludeCaster,             // everyone except region.caster
   }
   ```

   This is the **root-cause fix** for the Healing Circle bug. `update_aoe_regions` reads `targeting` and filters accordingly. Healing Circle becomes `CasterOnly`; Meteorite becomes `ExcludeCaster`.

### Phase 2 — Server cast-state component & systems

**Files:** `src/plugins/spells/components.rs`, `src/plugins/spells/systems.rs`, `src/plugins/spells/plugin.rs`

1. Add `CastProgress` component (see D3) in `components.rs`.

2. In `process_cast_requests`, branch on `cast_kind`:

   - **Instant** (default): unchanged path — call `spell.cast(ctx)` immediately, start cooldown, drain pending collections.
   - **CastTime / Channeling**: do **not** call `spell.cast` yet. Instead, spawn a `CastProgress` component on the caster. Validation (spellbook, cooldown ready, no existing cast) happens here; **cooldown is NOT started yet**. Pending damage/healing/aoe/etc. in `ctx` are ignored (instant portion is empty by contract for these spells).

3. New system `advance_cast_progress` (FixedUpdate, server-only):

   ```
   for each (caster_entity, mut cast, spellbook, mut cooldowns):
       spell = registry.get(cast.spell_id)
       cast.elapsed_seconds += delta

       match cast.kind:
           CastTime:
               if cast.elapsed_seconds >= cast.required_seconds:
                   build ctx with cast.target_position / target_entity
                   spell.cast(ctx)
                   drain ctx (same as instant path)
                   start cooldown
                   remove CastProgress (emit SpellCastEnded completed=true)

           Channeling:
               // Apply per-tick effect. Channeling spells scale their effect
               // off `ctx.caster_combat` and the time channeled so far
               // (see Phase 5 for the spell contract).
               build ctx
               spell.cast(ctx)        // appends incremental effects
               drain ctx
               // do NOT remove; do NOT start cooldown until release
   ```

   The "drain ctx" helper is extracted from `process_cast_requests` into a private `fn apply_spell_effects(commands, world resources, ctx)` so both paths share it.

4. New system `handle_cast_release` (server-only): consumes `SpellIntent::Release` messages, looks up the caster's `CastProgress` for that `spell_id`, and:
   - For Channeling: starts cooldown, removes `CastProgress`, emits `SpellCastEnded(completed=true)`.
   - For CastTime: removes `CastProgress` (cancelled), emits `SpellCastEnded(completed=false)`. No cooldown.

5. Interruptions: in `advance_cast_progress`, also check:
   - **Caster dead** → always cancel.
   - **CastTime + movement** → always cancel on any movement > epsilon. Hard rule, no per-spell toggle.
   - **Channeling + movement** → cancel only if `config.channel_movement == InterruptOnMove`. Swift uses `AllowMovement` so it keeps ticking while running.
   - **Channeling + re-press** → handled client-side by sending `Release` on the second press of the same key (see Phase 3); the server just consumes the release via `handle_cast_release`.
   - **Another `SpellIntent::Start` arrived for a different spell** → handled in `process_cast_requests` before spawning the new `CastProgress` (it removes the existing one and emits `SpellCastEnded { completed: false }`).

   Movement detection: store the caster's `Position` snapshot in `CastProgress` each tick and compare; `> 0.05` units of horizontal displacement counts as movement (tunable const `MOVEMENT_INTERRUPT_EPSILON`).

### Phase 3 — Protocol + client input

**Files:** `src/network/protocol.rs`, `src/network/client.rs`, `src/network/server.rs`

1. Add `SpellIntent` enum message on `Channel2` (`ClientToServer`), `SpellCastProgress` and `SpellCastEnded` on `Channel1` (`ServerToClient`). Register in `ProtocolPlugin`.

2. `src/network/server.rs::handle_spell_cast_commands` (rename or replace with `handle_spell_intents`):
   - Map each `SpellIntent::Start` to a `SpellCastRequest` (same as today). The downstream `process_cast_requests` decides whether to fire instantly or spawn `CastProgress`.
   - Map `SpellIntent::Release` to a new internal `SpellReleaseRequest` event consumed by `handle_cast_release`.

3. `src/network/client.rs::cast_spells_on_key`:
   - Track local channel state in a new `LocalChannelState` resource: `Option<SpellId>` of the currently held spell key.
   - On `keys.just_pressed(key)`:
     - If `key` matches the currently-channeling spell → send `SpellIntent::Release` and clear `LocalChannelState` (this is the D2c "re-press interrupts" rule).
     - Otherwise → send `SpellIntent::Start`. For instant and CastTime spells this is the only message; for channeling spells set `LocalChannelState = Some(spell_id)`.
   - On `keys.just_released(key)` for the currently channeling spell → send `SpellIntent::Release`, clear `LocalChannelState`.
   - For instant spells, `Release` is a no-op on the server.
   - Local prediction note: the client may optimistically render the bar fill before receiving the first replicated `SpellCastProgress` to avoid one-RTT lag; this is cosmetic and reconciled against authoritative state.

4. Server → client replication:
   - New system `replicate_cast_progress` (server-only) sends one `SpellCastProgress` per entity with `CastProgress` at a fixed cadence (every FixedUpdate tick; messages are ~32 bytes). Target `NetworkTarget::All` so **every** client (including the caster's own client) sees all active casts in the world.
   - `replicate_cast_end` sends `SpellCastEnded` (also `NetworkTarget::All`) whenever a `CastProgress` is removed (completed or interrupted).

### Phase 4 — Client cast bar UI (world-space + screen-space)

**File:** new `src/plugins/spells/cast_bar.rs`

Two distinct visual layers, both driven by replicated `SpellCastProgress`:

#### 4.1 World-space bar above every caster (primary, required)

- New resource `ObservedCasts` (client): `HashMap<u64 /* caster_network_id */, ObservedCast>` populated from `SpellCastProgress` messages and cleared on `SpellCastEnded`.
- New component `WorldCastBar { caster_network_id }` attached to a billboard UI node parented under (or tracking) the caster entity.
- System `sync_world_cast_bars`:
  1. Map each `ObservedCasts` entry to its local entity via the existing `NetworkEntityId` ↔ `Entity` query.
  2. Spawn / despawn `WorldCastBar` entities as casts start / end.
  3. Position the bar `Vec3::Y * 1.8` above the caster's `Transform` (billboard facing camera).
  4. Drive the fill width and label from the latest `SpellCastProgress`.
  5. For channeling spells (`kind == 1`), the bar visual differs: instead of filling toward a target, it pulses / shows elapsed seconds (since channeling is open-ended). A small "CH" icon distinguishes it from CastTime.
- This works for the **local player** too — their own bar is visible above their head just like everyone else's.

#### 4.2 Screen-space HUD bar (local player only, secondary, optional polish)

- A larger, fixed-position bar anchored at screen bottom-center (above the existing spell HUD), only for the local player's active cast. Same data source, just a bigger presentation for readability.
- Optional but recommended for UX: CastTime countdowns are easier to read on a fixed bar than on a billboard.

#### 4.3 End feedback

- On `SpellCastEnded`:
  - `completed=true` → brief green flash (200 ms) on the affected bar, then despawn.
  - `completed=false` → brief red flash + "Interrupted" label (400 ms), then despawn.

#### 4.4 Registration

Register the new systems in `spell_hud_systems` (or a new `cast_bar_systems(app)`) under `has_client` + `in_gameplay_or_paused` run conditions, mirroring the existing UI block.

**NetworkEntityId lookup note:** the existing `targeting` / `aoe` code already resolves `NetworkEntityId` to entities; reuse the same query pattern. The mapping is what makes the bar appear above the correct player for every observer.

### Phase 5 — Spell contract for channeling

Channeling spells need a clear contract so `advance_cast_progress` calling `spell.cast(ctx)` every tick produces sensible cumulative behaviour.

- The spell reads `ctx` for caster/target info and **emits incremental effects** each tick (e.g., 5 damage per 0.25s tick → 20 DPS while channeling).
- The spell must be idempotent in shape: each call must not stack permanent modifiers; use `ModifierEffect` with short durations or DoT/HoT-style per-tick healing/damage events.
- We define `Spell::channel_tick_interval_seconds(&self) -> f32` (default `0.25`) so the spell can choose how often to actually apply effects. `advance_cast_progress` only calls `spell.cast(ctx)` when the accumulator crosses the interval.

This keeps channeling spells data-driven and consistent with the existing `pending_*` drain pattern.

---

## Phase 6 — Fix Healing Circle (caster only)

**File:** `src/spells/healing_circle/definition.rs`

1. Change the `AoeEffect::ApplyModifier` payload to use `AoeTargeting::CasterOnly` (added in Phase 1).
2. No other change: the generic `update_aoe_regions` now filters targets so only `region.caster` matches.

**File:** `src/plugins/spells/aoe.rs`

In `update_aoe_regions`, after computing the candidate `target_entity`, add:

```rust
let allowed = match region.effect.targeting() {
    AoeTargeting::Everyone      => true,
    AoeTargeting::CasterOnly    => target_entity == region.caster,
    AoeTargeting::ExcludeCaster => target_entity != region.caster,
};
if !allowed { continue; }
```

(Accessor `targeting()` added to `AoeEffect` in Phase 1.)

This is the **root-cause fix** requested by the user; the alternative "caster-only via query filter" is more invasive and would not generalize to Meteorite's `ExcludeCaster` requirement.

---

## Phase 7 — New Spell: Meteorite

**Files:** new `src/spells/meteorite/{mod.rs,definition.rs,visual.rs}`

### 7.0 Player-facing flow (the contract)

```
t = 0.0s   Player presses T at a ground target location
           → Cast bar (1.0s) appears above the caster's head (visible to all clients)
           → Caster must stand still; moving cancels the cast

t = 1.0s   Cast completes
           → Red circle appears on the ground at the target location (radius = AREA_RADIUS)
           → The circle is a WARNING ZONE: anyone standing inside when the meteorite
             lands will be hit
           → No damage yet

t = 1.0s → 3.0s   Red circle stays visible for 2 seconds (the IMPACT_DELAY)
                  → Players have 2 seconds to walk OUT of the circle
                  → During the last ~0.6s, the meteorite rock becomes visible falling
                    from the sky toward the center of the circle

t = 3.0s   Meteorite impacts
           → 50 damage to EVERY entity inside the circle (radius check at impact moment)
           → EXCEPT the caster (AoeTargeting::ExcludeCaster)
           → Impact burst visual (expanding sphere + emissive flash)
           → Red circle + rock despawn
```

**Key clarification from the user:** the red circle is placed at the target location AFTER the cast completes, and stays for the full 2 seconds as a telegraphed warning. Damage is applied once, at impact, to whoever is inside at that moment. This is a "dodgeable AoE" pattern — enemies can walk out if they react in time. The caster is always immune to their own Meteorite.

### 7.1 Definition

```rust
pub struct MeteoriteSpell;

impl MeteoriteSpell {
    pub const ID: &'static str = "meteorite";
    pub const DISPLAY_NAME: &'static str = "Meteorite";
    pub const COOLDOWN_SECONDS: f32 = 8.0;
    pub const CAST_RANGE: f32 = 14.0;
    pub const AREA_RADIUS: f32 = 3.5;
    pub const CAST_TIME_SECONDS: f32 = 1.0;
    pub const IMPACT_DELAY_SECONDS: f32 = 2.0;   // red circle duration before impact
    pub const DAMAGE: f32 = 50.0;
}
```

`SpellConfig`:

```rust
SpellConfig::ranged_aoe(Self::COOLDOWN_SECONDS, Self::CAST_RANGE, Self::AREA_RADIUS)
    .with_cast_time(Self::CAST_TIME_SECONDS)   // helper added in Phase 1
```

`cast(&mut ctx)` (called server-side when the 1s cast completes):

- Compute `center = ctx.effective_center()` clamped to `CAST_RANGE` from caster.
- Spawn an AoE region with **two-phase behaviour**: a delay phase (red circle, no damage) followed by a single impact tick (damage burst).

  Add a new delayed-AoE primitive:
  - `AoeSpawnRequest::initial_delay_seconds` (default `0.0`) — the red circle is visible during this window but no effect applies.
  - `AoeRegion` stores `impact_pending_seconds: f32`; `update_aoe_regions` ticks it down and only applies `effect` once it reaches `0.0`.
  - The effect itself is `AoeEffect::Damage { amount: 50.0, targeting: ExcludeCaster }` with `once_per_entity = true`, and the AoE region despawns immediately after the impact tick.

- Extend `AoeEffect` with damage/heal burst variants (the current `ApplyModifier` only handles modifiers):

  ```rust
  AoeEffect::Damage { amount: f32, targeting: AoeTargeting }
  AoeEffect::Heal   { amount: f32, targeting: AoeTargeting }
  ```

  `update_aoe_regions` matches these and emits `DamageEvent` / `HealEvent`. With `once_per_entity = true` + immediate despawn after impact, damage fires exactly once at the moment of impact. This is the **right primitive** for Meteorite and unblocks any future "bomb"-style AoE.

- Emit `ctx.emit_visual("meteorite", center, center)` so the client spawns the red circle marker + falling rock (see 7.2).

### 7.2 Visual (`src/spells/meteorite/visual.rs`)

Three-phase visual, all spawned from a single `SpellVisualEffect` message on impact of the cast:

1. **Red warning circle** (`t = 0` to `t = IMPACT_DELAY` = 2.0s): a flat red glowing ring/cylinder on the ground at `center`, `AREA_RADIUS` wide. Pulses gently (scale + alpha oscillation) so it reads as "danger". This is the telegraph — players see exactly where the meteorite will land and can walk out.
2. **Falling rock** (last ~0.6s before impact): a `Cuboid` or `Sphere` (dark, emissive orange/red) descending from `center + Vec3::Y * 20` to `center + Vec3::Y * 0.5`. Scales up slightly as it approaches the ground for depth perception.
3. **Impact burst** (0.2s after rock lands): rapidly expanding sphere + brief bright emissive flash, then despawn all meteorite visual entities.

The visual timeline is driven client-side by a local timer component `MeteoriteVisual { elapsed_seconds }` started when the `SpellVisualEffect` is received. The client does **not** wait for a second network message for the impact — it knows `IMPACT_DELAY` from a shared constant and animates autonomously. The server independently applies damage at `IMPACT_DELAY`; the visuals are cosmetic.

The dispatcher in `src/plugins/spells/effects.rs::spawn_spell_visuals` gets a new match arm for `MeteoriteSpell::ID`, and `client_effect_systems` registers `crate::spells::meteorite::visual::animate` next to the existing ones.

### 7.3 Registration & keybind

- `src/plugins/spells/systems.rs::register_builtin_spells`: register `Arc::new(MeteoriteSpell)`.
- `src/spells/mod.rs`: add `pub mod meteorite;`.
- `src/plugins/key_mapping.rs`: bind `meteorite` to `KeyCode::KeyT` (confirmed).
- `src/plugins/spells/ui.rs::key_label`: add `KeyCode::KeyT => "T"`.

### 7.4 Reasoning for the user

The flow matches exactly what was requested: cast 1s (stand still) → red circle appears → 2s later meteorite falls → 50 damage to everyone in the circle except the caster. The red circle is a **dodgeable telegraph**: if enemies react within the 2s window they can walk out and take no damage. This is a more interesting spell than an instant nuke because it rewards positional awareness. The caster's immunity (`ExcludeCaster`) prevents accidental suicide even if they stand in their own circle.

The server is authoritative for both the timing and the damage; the client visual is cosmetic and self-timed from the moment it receives the cast-completed visual message. Damage is computed at impact against the **current** positions of entities, not their positions at cast time — so the 2s window genuinely matters.

---

## Phase 8 — New Spell: Swift (key F, channeling, +20% MS)

**Files:** new `src/spells/swift/{mod.rs,definition.rs,visual.rs}`

### 8.1 Definition

```rust
pub struct SwiftSpell;

impl SwiftSpell {
    pub const ID: &'static str = "swift";
    pub const DISPLAY_NAME: &'static str = "Swift";
    pub const COOLDOWN_SECONDS: f32 = 0.0;            // none: the channel itself is the cost
    pub const SPEED_MULTIPLIER: f32 = 1.20;           // +20% movement speed
    /// How often the spell refreshes the MS modifier while channeling.
    /// The modifier has a short expiration so it auto-clears on release.
    pub const TICK_INTERVAL_SECONDS: f32 = 0.25;
    pub const MODIFIER_DURATION_SECONDS: f32 = 0.5;   // > TICK so it never gaps
}
```

This is a **channeling self-buff**. While F is held, the caster gains +20% MS. Releasing F (or being interrupted) ends the buff within `MODIFIER_DURATION_SECONDS`.

`SpellConfig`:

```rust
SpellConfig::new(Self::COOLDOWN_SECONDS, 0.0, 0.0, TargetingMode::SelfCentered)
    .with_channel(ChannelMovementPolicy::AllowMovement)  // can run while channeling
```

`cast(&mut ctx)` (called every channel tick by `advance_cast_progress`):

```rust
ctx.emit_modifier(
    ctx.caster,
    vec![ModifierEffect::MovementSpeedMultiplier(Self::SPEED_MULTIPLIER)],
    Some(Self::MODIFIER_DURATION_SECONDS),
    ModifierKind::Buff,
);
```

The modifier's duration is longer than the tick interval, so while channeling the buff stays continuously active; when the channel stops, no new ticks are emitted and the buff falls off within `MODIFIER_DURATION_SECONDS`.

`channel_tick_interval_seconds()` returns `Self::TICK_INTERVAL_SECONDS`.

### 8.2 Stat modifier support

**Verify** (do not assume) that `ModifierEffect` already has a `MovementSpeedMultiplier(f32)` variant. If it does not, add it in `src/stats/events.rs` and implement its application in `src/plugins/player_movement.rs` (multiply effective move speed by the active modifier). Existing `ApplyStatModifierEvent` flow already handles short-lived modifiers with expiry, so the tick-refresh pattern composes naturally.

### 8.3 Visual

A faint boot / feet aura under the player while channeling (re-emitted on a longer cadence than the stat tick to avoid clutter, e.g., every 0.5s). Mark with `SpellVisual` for cleanup. While channeling is active the world-space bar above the player shows the "CH" indicator (Phase 4.1).

### 8.4 Registration & keybind

- `register_builtin_spells`: register `SwiftSpell`.
- `src/spells/mod.rs`: add `pub mod swift;`.
- `src/plugins/key_mapping.rs`: bind `swift` to `KeyCode::KeyF`.
- `src/plugins/spells/ui.rs::key_label`: add `KeyCode::KeyF => "F"`.

### 8.5 Reasoning for the user

Channeling was the right fit for the movement buff: "piú tempo channeli piú hai il buff" maps directly to "refresh the modifier each tick while held." Using `ChannelMovementPolicy::AllowMovement` is essential — a speed buff you can't use while moving would be pointless. The re-press interrupt (D2c) gives the player a fast way to cancel without scrambling for the release edge case.

---

## Resolved Decisions (from user feedback)

1. **Meteorite keybind** → `T` (confirmed).
2. **Movement interruption** — CastTime: always interrupted by movement. Channeling: per-spell via `ChannelMovementPolicy` (Swift = `AllowMovement`, offensive channels can use `InterruptOnMove`).
3. **Channeling spell shipped** → Swift (F) is the channeling spell. Holding F channels; releasing or re-pressing F ends the channel.
4. **Movement speed buff amount** → **+20%** (`SPEED_MULTIPLIER = 1.20`).
5. **Re-press during channel** → interrupts (D2c, hard rule).
6. **World-space cast bars** → required for ALL players (local + remote), driven by replicated `SpellCastProgress`. Network-ready by design.
7. **Healing Circle** → `AoeTargeting::CasterOnly` (confirmed "only the caster").

## Remaining Open Questions

1. **Movement speed buff stacking** — if multiple Swift-like effects coexist, should multipliers multiply (current proposal) or add? Default to the stats module's existing policy.
2. **Channeling bar visual for open-ended channels** — for Swift there's no `required_seconds`. Proposal: bar pulses and shows elapsed time, with a distinct color vs CastTime. Confirm acceptable.
3. **Screen-space HUD bar (Phase 4.2)** — optional polish for the local player. Confirm whether to include it or rely solely on the world-space bar.

---

## File-by-File Change Summary

| File                                                                  | Change                                                                                                  |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `src/plugins/spells/context.rs`                                       | `SpellConfig` cast/channel fields, `CastKind`, `AoeTargeting`, `AoeEffect::Damage/Heal`, `channel_tick_interval_seconds` |
| `src/plugins/spells/components.rs`                                    | New `CastProgress` component                                                                            |
| `src/plugins/spells/systems.rs`                                       | Branch on cast kind, extract `apply_spell_effects`, add `advance_cast_progress`, `handle_cast_release`, register Meteorite + Swift |
| `src/plugins/spells/plugin.rs`                                        | Register new systems in `FixedUpdate` chain (server-only)                                               |
| `src/plugins/spells/aoe.rs`                                           | `AoeTargeting` filter, `impact_pending_seconds` delay, `Damage`/`Heal` payload branches                  |
| `src/plugins/spells/mod.rs`                                           | Re-exports for new types                                                                                |
| `src/plugins/spells/effects.rs`                                       | New match arm for Meteorite + Swift visuals, register their `animate` systems                           |
| `src/plugins/spells/ui.rs`                                            | Add `KeyF` / `KeyT` labels                                                                              |
| `src/plugins/spells/cast_bar.rs` (new)                                | World-space + screen-space cast bar UI (replicated, all players)                                        |
| `src/network/protocol.rs`                                             | `SpellIntent`, `SpellCastProgress`, `SpellCastEnded` messages                                           |
| `src/network/client.rs`                                               | Send `Start`/`Release`, track local channel state for `Release`                                         |
| `src/network/server.rs`                                               | Handle `SpellIntent::Release`, add `replicate_cast_progress`, `replicate_cast_end`                      |
| `src/plugins/key_mapping.rs`                                          | Bind `meteorite` → `T`, `swift` → `F`                                                                   |
| `src/spells/mod.rs`                                                   | `pub mod meteorite; pub mod swift;`                                                                     |
| `src/spells/healing_circle/definition.rs`                             | `AoeTargeting::CasterOnly`                                                                              |
| `src/spells/meteorite/{mod.rs,definition.rs,visual.rs}` (new)         | 1s cast time, 2s delayed impact, 50 AoE dmg, `ExcludeCaster`                                            |
| `src/spells/swift/{mod.rs,definition.rs,visual.rs}` (new)             | Channeling +20% MS buff, `AllowMovement`, re-press interrupts                                           |
| `src/stats/events.rs` (if needed)                                     | `ModifierEffect::MovementSpeedMultiplier`                                                               |
| `src/plugins/player_movement.rs` (if needed)                          | Apply active MS modifiers                                                                               |

---

## Suggested Implementation Order

1. **Phase 1** — context/`SpellConfig`/`AoeTargeting`/`ChannelMovementPolicy` extensions (compiles, no behaviour change).
2. **Phase 6** — Healing Circle fix (smallest user-visible win; unblocks Phase 7).
3. **Phase 2 + Phase 3** — server cast state + protocol messages (foundations, no content yet).
4. **Phase 4** — world-space + screen-space cast bar UI (now testable with any test CastTime spell).
5. **Phase 5 + Phase 8** — channeling contract + Swift spell (first shippable channeling content).
6. **Phase 7** — Meteorite (puts everything together: cast time + delayed AoE + `ExcludeCaster` + bar UI).

Each phase compiles and is testable in isolation. The user can stop after any phase and still have a working game.

---

## Validation Strategy

- `cargo check` after each phase.
- `cargo test` — existing spell tests in `fireball/definition.rs` and `ui.rs` must still pass.
- Manual smoke test in-game (single-player + 2-client multiplayer):
  1. Heal Circle heals only the caster (stand near an enemy dummy, both inside the circle → dummy HP unchanged).
  2. **Hold F** → +20% MS buff active, world-space bar appears above the local player's head with "CH" indicator. **Other clients see the same bar above the player.** Move while channeling → bar persists (Swift = `AllowMovement`).
  3. **Release F** → bar disappears within ~0.5s, MS returns to baseline. **Re-press F** mid-channel → same effect (channel interrupted, then a new one could start on next press per key-handling rules).
  4. Press T (Meteorite) → cast bar fills 1.0s above caster's head (visible to all clients) → ground marker appears → meteorite falls after 2s → all dummies take 50 damage, caster takes 0.
  5. Walk during Meteorite cast → bar flashes red and disappears for ALL clients (interrupted).
  6. Verify a second client also sees Swift's bar and Meteorite's bar above the respective casters, with correct fill states.
- Verify no regressions on existing instant spells (Attack, Fireball, Followball).
