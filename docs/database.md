# Database

There isn't one, in the usual sense. There is no ORM, no connection pool, no migration files, and no persistence layer — because SpacetimeDB collapses three things the previous stack kept apart.

Under Postgres, every piece of player state had to be written three times: as a Bevy component, as a network protocol message, and as a table plus an entity plus a `load_*`/`save_*` pair. Adding `KnownGlyphs` meant a migration, an entity, two repository methods, a protocol registration and a component. Here, a table row **is** the authoritative state, **is** the persisted record, and **is** what gets replicated to subscribed clients. Declaring it once is the whole job.

## Where the schema lives

`crates/stdb-module/src/tables.rs`. That file is the single source of truth: the reducers are written against it, and the client's bindings are generated from it.

## Working with it

```sh
docker compose up -d spacetimedb          # start the instance on :3000
./scripts/stdb.sh publish                 # compile the module to WASM and upload
./scripts/stdb.sh generate                # regenerate the client bindings
./scripts/stdb.sh sql "SELECT * FROM player"
./scripts/stdb.sh logs
./scripts/stdb.sh reset                   # wipe and re-seed — destructive
```

`./scripts/stdb.sh dev` does watch + rebuild + republish + regenerate in one command, and is the loop to use while working on the module.

## Migrations

Publishing applies schema changes automatically. Adding a column with a default, adding a table, adding an index: all handled. A change SpacetimeDB cannot migrate automatically will be refused at publish time, and the answer is `reset`, which destroys the data.

`reset` is also how you re-run `init`. `init` seeds the world and only fires against an *empty* database, so after changing world seeding you need a reset to see the effect.

## Persistent versus runtime tables

SpacetimeDB persists everything. Nothing distinguishes "this row should survive a restart" from "this row is a projectile in flight" — that distinction is one the module keeps for itself, and `init` enforces it by clearing the runtime tables before re-seeding.

| Persistent | Runtime (cleared by `init`) |
| --- | --- |
| `player`, `player_stats`, `hotbar`, `inventory`, `equipment`, `known_glyphs`, `prop_override` | `game_entity` (non-players), `entity_stats`, `cast_state`, `cooldown`, `projectile`, `aoe_region`, `crowd_control`, `stat_modifier`, `threat`, `boss_state`, `tick_stats` |

Forgetting this is how a republish inherits yesterday's mid-flight projectiles.

## Event tables

`spell_visual_effect`, `damage_event`, `cast_ended` and `player_message` are declared `event`. Rows are delivered to subscribers and not retained — the right lifetime for "play this effect once". They replace the server-to-client messages the lightyear protocol used to carry, and they exist because SpacetimeDB 2.0 removed global reducer callbacks.

## Two constraints worth knowing before you edit the schema

**Row types need named fields.** `#[derive(SpacetimeType)]` panics on tuple structs — `spacetimedb-bindings-macro` calls `f.ident.unwrap()` on every field, and an unnamed field has no ident. This is why `bevymmo_domain`'s newtypes (`SpellId`, `ItemId`, `EntityId`) are mirrored in `crates/stdb-module/src/rows.rs` instead of stored directly.

**SATS has no impl for `[T; N]`, `Cow` or `HashMap`.** Fixed-size arrays become `Vec` with the length carried by convention, `Cow<'static, str>` becomes `String`, and maps become either a `Vec` or — usually better — a separate table with an index.

## Spatial queries

There is no ECS to query, so "every entity within range" is an index scan. `game_entity` carries `cell_x`/`cell_z` (see `GRID_CELL_SIZE` and `grid_cell`) precisely for that: a linear scan per mob per tick does not survive contact with a populated map.

## Authentication

A connection's `Identity` is issued and verified by SpacetimeDB, and characters are keyed by it. This closed a real hole: the Postgres schema keyed characters on a normalised name while netcode ran with an all-zero private key, so anyone who knew a name could log in as that character.

The client caches its token next to the user settings. Deleting it means a new identity, and therefore a new character.
