# Smoke Test Checklist — refactor safety net

Used to validate that every slice of `plans/workspace-crate-split.md` keeps
observable behavior intact. Run the full checklist after each slice.

**Baseline recorded on 2026-08-06 (Slice 0)**:

| Check | Result |
|---|---|
| `cargo test` | ✅ 162 passed, 0 failed |
| `cargo clippy -- -D warnings` | ⚠️ Fails: **73 pre-existing errors** (baseline, not introduced by refactor) |
| Headless server build (`--no-default-features --features server,netcode,udp,replication`) | ✅ Compiles (46 pre-existing warnings) |

> The clippy failures are **pre-existing** and intentionally out of scope for
> the crate-split refactor. Do not fix them in a refactor slice: they belong to
> a separate cleanup slice. The invariant is: *the refactor must not introduce
> NEW clippy warnings*.

---

## Prerequisites

- PostgreSQL running locally (`docker compose up -d`).
- `config/local.toml` exists with `DATABASE_URL` (or env var set).

## Checklist

### 1. Dedicated server starts headless

- [ ] `cargo run -- server` starts without a window.
- [ ] Log shows server listening on the configured bind address.
- [ ] No `Mesh`, `Scene`, or renderer systems are registered (server is headless).

### 2. Two clients connect and spawn

- [ ] `cargo run -- client --client-id 101` connects.
- [ ] `cargo run -- client --client-id 102` connects.
- [ ] Each client sees its own player entity spawned by the server.
- [ ] Each client sees the other player (replication works).

### 3. Movement

- [ ] Right-click moves the player (click indicator ring appears).
- [ ] The other client sees the movement (replicated `Position`).
- [ ] Prediction is smooth (no teleporting back on server correction).

### 4. Scoreboard

- [ ] Holding Tab lists both players with names.
- [ ] Player 101 disconnects → scoreboard updates without leaving 101 listed.

### 5. Combat & death/respawn

- [ ] Casting a spell damages an enemy (health bar updates on both clients).
- [ ] Enemy death transitions to `Dead` state; UI reflects it.
- [ ] Respawn brings the enemy back (respawn system works).

### 6. Host-client mode

- [ ] `cargo run -- host-client --client-id 201` starts with a window.
- [ ] Game screen spawns (camera, light, ground).
- [ ] Player can move and cast (embedded server + client both work).

## After each slice

| Command | Expected |
|---|---|
| `cargo test` | Same count or more; 0 failures |
| `cargo fmt --check` | Clean |
| `cargo check --workspace` | Clean (Slice 1+) |
| Headless server build | Compiles; no `presentation` in `cargo tree` (Slice 3+) |
| Clippy | No NEW warnings vs the 73-error baseline |
