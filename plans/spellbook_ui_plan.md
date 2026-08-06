## Goal Description

Implement a new client-side "Spellbook" UI that lets players assign spells explicitly to the `Q`, `W`, and `E` hotbar slots. The runtime gameplay model will represent these three cast slots directly instead of relying on the current generic `Spellbook.spells + KeyBindings.spells` mapping.

The UI is called "Spellbook" because it is the player-facing panel, but the ECS/database state should use the domain term **hotbar** for the three equipped slots. This avoids confusing "known/unlocked spells" with "currently assigned cast keys".

## User Review Required

> [!IMPORTANT]
> This plan intentionally avoids editing already-applied migrations like `m20260805_000003_create_player_spells.rs`. Instead, it adds a new migration that creates `player_hotbar`, backfills from `player_spells`, and then removes the old table if we decide this is the final schema.
>
> Decision needed before implementation: should `player_spells` be removed permanently in the new migration, or should we keep it because future progression/unlocks will need a separate "known spells" table?

## Proposed Changes

---

### Domain Model: Hotbar State (`src/plugins/spells/components.rs`)

Introduce an explicit hotbar component for the three cast slots.

#### [MODIFY] `src/plugins/spells/components.rs`

Add a slot enum and hotbar component:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HotbarSlot {
    Q,
    W,
    E,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpellHotbar {
    pub q_spell: Option<SpellId>,
    pub w_spell: Option<SpellId>,
    pub e_spell: Option<SpellId>,
}
```

Add helper methods to keep slot logic centralized:

```rust
impl SpellHotbar {
    pub fn spell_for_slot(&self, slot: HotbarSlot) -> Option<&SpellId>;
    pub fn assign(&mut self, slot: HotbarSlot, spell_id: Option<SpellId>);
    pub fn contains(&self, spell_id: &SpellId) -> bool;
    pub fn assigned_slots(&self) -> impl Iterator<Item = (HotbarSlot, &SpellId)>;
}
```

Replace the current default spell list with a slot-aware default:

```rust
pub fn default_player_hotbar() -> SpellHotbar {
    SpellHotbar {
        q_spell: Some(SpellId::new("attack")),
        w_spell: Some(SpellId::new("fireball")),
        e_spell: Some(SpellId::new("healing_circle")),
    }
}
```

#### Design note

Prefer `SpellHotbar` over changing `Spellbook` to contain `q_spell`, `w_spell`, and `e_spell`. `Spellbook` should remain available as a future concept for known/unlocked spells if progression is added later.

If implementation scope should stay smaller, `Spellbook` can be replaced entirely by `SpellHotbar`, but the type name should still be changed to avoid semantic drift.

---

### Database Migration (`src/migrations/`)

Create a new migration instead of rewriting existing migration history.

#### [NEW] `src/migrations/m20260806_000006_create_player_hotbar.rs`

Create `player_hotbar`:

```rust
manager
    .create_table(
        Table::create()
            .table(PlayerHotbar::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(PlayerHotbar::PlayerId)
                    .uuid()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(PlayerHotbar::QSpell).text())
            .col(ColumnDef::new(PlayerHotbar::WSpell).text())
            .col(ColumnDef::new(PlayerHotbar::ESpell).text())
            .foreign_key(
                ForeignKey::create()
                    .name("fk-player_hotbar-player_id")
                    .from(PlayerHotbar::Table, PlayerHotbar::PlayerId)
                    .to(Players::Table, Players::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .to_owned(),
    )
    .await?;
```

Backfill from `player_spells` using slot order:

```sql
INSERT INTO player_hotbar (player_id, q_spell, w_spell, e_spell)
SELECT
    player_id,
    MAX(CASE WHEN slot_index = 0 THEN spell_id END) AS q_spell,
    MAX(CASE WHEN slot_index = 1 THEN spell_id END) AS w_spell,
    MAX(CASE WHEN slot_index = 2 THEN spell_id END) AS e_spell
FROM player_spells
GROUP BY player_id
ON CONFLICT (player_id) DO NOTHING;
```

For players without existing rows, insert defaults:

```sql
INSERT INTO player_hotbar (player_id, q_spell, w_spell, e_spell)
SELECT id, 'attack', 'fireball', 'healing_circle'
FROM players
WHERE NOT EXISTS (
    SELECT 1 FROM player_hotbar WHERE player_hotbar.player_id = players.id
);
```

Then, depending on the review decision:

- final hotbar-only schema: drop `player_spells` in this migration;
- future unlock-ready schema: keep `player_spells` and treat it as known/unlocked spell ownership.

#### [MODIFY] `src/migrations/mod.rs`

Register the new migration after `m20260806_000005_rename_followball_spell_to_fireball`.

#### [NO CHANGE] Existing migrations `000003`, `000004`, `000005`

Do not rewrite already-versioned migrations unless we explicitly choose a full local database reset workflow.

---

### Persistence Entities (`src/plugins/persistence/entity/`)

#### [NEW] `src/plugins/persistence/entity/player_hotbar.rs`

Add a SeaORM entity for `player_hotbar`:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "player_hotbar")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub player_id: Uuid,
    pub q_spell: Option<String>,
    pub w_spell: Option<String>,
    pub e_spell: Option<String>,
}
```

#### [MODIFY] `src/plugins/persistence/entity/mod.rs`

Export `player_hotbar`.

#### [CONDITIONAL DELETE] `src/plugins/persistence/entity/player_spell.rs`

Delete only if the new migration permanently drops `player_spells`. If we keep known/unlocked spells, retain this entity and separate repository methods for spell ownership from hotbar assignment.

---

### Server Persistence (`src/plugins/persistence/repository/player.rs`)

Update player snapshots to load/save `SpellHotbar`.

#### [MODIFY] `src/plugins/persistence/repository/player.rs`

- Replace or supplement `spellbook: Spellbook` in `PersistedPlayerSnapshot` with `hotbar: SpellHotbar`.
- Add `load_hotbar(player_id)`.
- Add `save_hotbar(player_id, hotbar)`.
- Add `load_or_create_default_hotbar(player_id)`.
- Use single-row update/insert instead of delete-many/insert-many.

The repository should preserve the existing async boundary: it must be called from the persistence runtime/task pattern, not awaited inside synchronous Bevy systems.

---

### Network Protocol (`src/network/protocol.rs`)

Add a small command message for changing one hotbar slot.

#### [MODIFY] `src/network/protocol.rs`

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UpdateHotbarSlotRequest {
    pub slot: HotbarSlot,
    pub spell_id: Option<String>,
}
```

Register it in `ProtocolPlugin` as `ClientToServer`.

#### Rationale

A slot-level command matches the UI interaction (`Assign Q`, `Assign W`, `Assign E`) and avoids overwriting slots the user did not touch. If a full save/apply flow is desired later, add a separate `UpdateHotbarRequest` with all three slots.

---

### Server Logic (`src/network/server.rs`)

Handle hotbar update requests server-authoritatively.

#### [MODIFY] `src/network/server.rs`

Add `handle_update_hotbar_slot_requests` to the server update chain.

The system must:

1. read `UpdateHotbarSlotRequest` from connected/joined clients;
2. resolve the player entity from `RemoteId`/`PlayerId`;
3. validate `spell_id` against `SpellRegistry` when `Some`;
4. optionally validate that the player knows/unlocked the spell if `Spellbook` remains as a separate ownership component;
5. update the authoritative `SpellHotbar` component;
6. persist via `PlayerRepository::save_hotbar` using `PersistenceRuntime`;
7. never trust the client as authoritative source of ECS state.

Duplicate policy to implement:

- `None` clears the requested slot.
- `Some(spell)` assigns that spell to the requested slot.
- If the same spell exists in another slot, remove it from the old slot so a spell appears at most once in the hotbar.

Persistence must run asynchronously using the existing `PlayerStore` + `PersistenceRuntime` pattern. Do not block a Bevy system on database I/O.

---

### Network Replication (`src/network/protocol.rs`)

#### [MODIFY] `src/network/protocol.rs`

Replicate the new hotbar component:

```rust
app.component::<SpellHotbar>().replicate().predict();
```

Remove `Spellbook` replication only if `Spellbook` is fully replaced. If `Spellbook` remains as known/unlocked spell ownership, decide whether it should be replicated to clients or kept server-only.

---

### Client Input Integration (`src/network/client.rs`)

#### [MODIFY] `src/network/client.rs`

Replace the current `cast_spells_on_key` loop over `spellbook.spells` and `KeyBindings.spells` with explicit slot lookup:

- `KeyCode::KeyQ` -> `hotbar.q_spell`
- `KeyCode::KeyW` -> `hotbar.w_spell`
- `KeyCode::KeyE` -> `hotbar.e_spell`

Keep the existing behavior for:

- target position calculation;
- current target lookup;
- cooldown gating through `SpellHudState`;
- sending `SpellCastCommand` over `Channel2`;
- local cooldown start for instant/channeling spells.

Also update any release/cancel logic if channeling or cast-time release input is currently tied to the old spell/key mapping.

---

### Spell Casting Validation (`src/plugins/spells/systems.rs`)

#### [MODIFY] `src/plugins/spells/systems.rs`

Update server-side cast validation to check the hotbar:

```rust
if !hotbar.contains(&request.spell_id) {
    bevy::log::warn!("Caster attempted to cast a spell not assigned to the hotbar");
    continue;
}
```

If `Spellbook` remains as a separate known/unlocked spell list, validate both:

1. player knows/unlocked the spell;
2. spell is assigned to Q/W/E.

---

### Spell Registry UI Support (`src/plugins/spells/registry.rs`)

#### [MODIFY] `src/plugins/spells/registry.rs`

Add a deterministic iteration API so the Spellbook UI can list all available spells without relying on `HashMap` order.

Example API:

```rust
pub fn sorted_spells(&self) -> Vec<(SpellId, Arc<dyn Spell>)>;
```

Sort by display name or spell id. Prefer display name for UI stability/readability.

---

### Spellbook UI Implementation (`src/ui/spellbook/`)

Create the player-facing Spellbook panel using the existing UI plugin folder pattern.

#### [NEW] `src/ui/spellbook/mod.rs`
#### [NEW] `src/ui/spellbook/plugin.rs`
#### [NEW] `src/ui/spellbook/systems.rs`
#### [NEW] `src/ui/spellbook/components.rs`

The plugin should:

- run only on clients using `network::mode::has_client`;
- toggle the panel with `K`;
- list spells from `SpellRegistry::sorted_spells()`;
- show assign buttons for `Q`, `W`, and `E`;
- send `UpdateHotbarSlotRequest` on click;
- avoid mutating authoritative replicated hotbar state directly as the source of truth;
- optionally apply an optimistic local visual state only if it is easy to reconcile with replication.

#### [MODIFY] `src/ui/mod.rs`
#### [MODIFY] `src/ui/plugin.rs`

Export and register `spellbook::SpellbookPlugin` with the main `UiPlugin`.

---

### HUD Integration (`src/plugins/spells/ui.rs`)

#### [MODIFY] `src/plugins/spells/ui.rs`

Remove dependency on `KeyBindings.spells`.

Render fixed hotbar slots instead of rendering arbitrary spell/key pairs:

- `Q`: assigned spell display name or `Empty`;
- `W`: assigned spell display name or `Empty`;
- `E`: assigned spell display name or `Empty`.

Cooldown labels should remain keyed by `SpellId`, but layout identity should be based on `(HotbarSlot, Option<SpellId>)` so changing an assignment rebuilds the HUD correctly.

---

### Keybindings Integration (`src/plugins/key_mapping.rs`)

#### [MODIFY] `src/plugins/key_mapping.rs`

Remove the hardcoded spell map:

```rust
pub spells: HashMap<SpellId, KeyCode>
```

Keep global bindings such as:

- `show_scoreboard`;
- `toggle_pause`.

If desired, add a future `open_spellbook` keybinding for `KeyCode::KeyK`, but the hotbar cast keys themselves should remain fixed to Q/W/E for this feature.

---

### Documentation and Naming Cleanup

#### [MODIFY] touched Rust docs/comments

- Document new non-trivial structs/functions with `///` docs.
- Explain why `SpellHotbar` is separate from UI "Spellbook" naming.
- Keep docs in English for new/changed APIs.
- Avoid deep `super::super` paths; import symbols or use `crate::...` paths.

---

## Verification Plan

### Automated Tests

Run:

```bash
cargo test
cargo clippy -- -D warnings
```

Add or update focused tests where practical:

- `SpellHotbar::assign` removes duplicates from other slots;
- `SpellHotbar::contains` checks all slots;
- repository `load_hotbar` creates defaults when absent;
- migration backfill maps first three old `slot_index` values to Q/W/E.

### Manual Verification

1. Start local Postgres if needed:
   ```bash
   docker compose up -d
   ```
2. Run:
   ```bash
   cargo run -- host-client
   ```
3. Join with a player.
4. Verify the HUD shows Q/W/E slots.
5. Press `K` to open the Spellbook UI.
6. Assign different spells to Q, W, and E.
7. Verify pressing Q/W/E casts the assigned spells.
8. Verify assigning the same spell to a different slot removes it from the old slot.
9. Clear a slot and verify the corresponding key does not cast.
10. Disconnect/reconnect and verify the hotbar persists.

### Migration Verification

For an existing local database with `player_spells`:

1. Run the server once so migrations apply automatically.
2. Confirm `player_hotbar` exists.
3. Confirm existing players have Q/W/E populated from `slot_index` 0/1/2.
4. Confirm players without old spell rows receive defaults.

For a clean database:

1. Run the server once.
2. Confirm new players receive default hotbar assignments.

If we choose to drop `player_spells`, no database reset should be required because the new migration performs the schema transition.