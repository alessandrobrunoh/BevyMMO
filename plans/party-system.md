# Plan: Party System

**Branch**: TBD
**Status**: Decisions confirmed (2026-08-17) — ready to start Slice 1
**Depends on / relates to**: [`plans/account-chat-admin.md`](./account-chat-admin.md) — reuses its chat input, `player_message` notification path, and conventions. Does **not** depend on that plan's Slice 3 (persistent roles) or Slice 5 (generic `/`-command dispatcher); see Decision 1.

## Goal

Let players form a party of up to 5 through `/party` chat commands, see who is in their party, and be protected from friendly-fire damage while grouped: `/party invite`, `/party join`, `/party accept`, `/party decline`, `/party leave`, `/party list`, and bare `/party` for help.

## Current architecture and constraints (verified in codebase, 2026-08-17)

- `Player` (`crates/stdb-module/src/tables.rs:57`) is keyed by `spacetimedb::Identity`, with a `#[unique] normalized_name`, `display_name`, and a `#[unique] entity_id: u64` linking to the combat-side `GameEntity`. No party/friend/group table exists anywhere in the schema today — this plan adds new tables from scratch.
- `crates/stdb-module/src/sim/combat.rs::apply_damage(ctx, target: u64, source: Option<u64>, amount: f32)` (~line 348) is the **single shared entry point for all damage** (spells, DoTs, AI). It has **no PvP/PvE authorization check at all** today — no comparison of attacker vs target ownership. `EntityKindRow` (`Player | Enemy | Boss | Dummy | Npc`) already exists and lets us cheaply tell "is this entity a player" before doing any party lookup.
- `crates/stdb-module/src/reducers/chat.rs` has exactly one reducer, `send_chat_message`, and **no `/`-prefix command parser or dispatcher exists yet** on the server. `plans/account-chat-admin.md` Slice 5 plans a generic typed command dispatcher for `/kill`/`/give`, but it is not built. Per Decision 1 below, party commands do not wait for it.
- Targeted, single-player notifications already exist via `PlayerMessageEvent { target: Option<Identity>, text: String }` (`crates/stdb-module/src/tables.rs:579`), inserted e.g. by `combat.rs::respawn`. The client (`crates/client/src/stdb/plugin.rs` ~line 765) renders any row where `target.is_none() || target == local_identity` into both the chat log and the notice log. Party invite/request notifications will reuse this path.
- The bottom-left chat widget (`crates/presentation/src/ui/chat.rs`) and the lower-left notice log (`crates/presentation/src/ui/notices/`) both follow the same shape: a `Resource` holding the root `Entity`, a marker `Component` per line, a `Startup` spawn system, and chained `Update` systems reading a `MessageReader<T>`. New party UI feedback should follow this same shape rather than introducing a different pattern.
- `stdb-module` reducer tests are inline `#[cfg(test)] mod tests` blocks at the bottom of the file under test (e.g. `chat.rs:48-63`), typically exercising pure helper functions rather than a full mocked `ReducerContext`/DB harness. `parties.rs` should follow this convention: pure validation/resolution helpers get direct unit tests; anything needing `ReducerContext` is covered as far as the existing test harness allows, consistent with how `chat.rs` and `combat.rs` are tested today.
- No persistent role/permission system exists yet (`plans/account-chat-admin.md` Slice 3 is not started); this plan does not need one — party actions are self-service (a player only ever acts on their own membership/requests).
- `plans/account-chat-admin.md` Decision 10 wipes the current SpacetimeDB database immediately before that plan's Slice 1 ships. New tables added by this plan carry no pre-existing data, so there is no migration concern either way.

## Proposed architecture

Three new public tables in `crates/stdb-module/src/tables.rs`:

```rust
#[table(accessor = party, public)]
pub struct PartyRow {
    #[primary_key]
    #[auto_inc]
    pub party_id: u64,
    pub leader: Identity,
    pub created_at: Timestamp,
}

#[table(accessor = party_member, public)]
pub struct PartyMemberRow {
    #[primary_key]
    pub identity: Identity,   // a player can be a member of at most one party at a time
    pub party_id: u64,
    pub joined_at: Timestamp,
}

#[derive(SpacetimeType, Clone, Copy, PartialEq, Eq)]
pub enum PartyRequestKind {
    Invite,       // leader -> target
    JoinRequest,  // outsider -> leader
}

#[table(accessor = party_request, public)]
pub struct PartyRequestRow {
    #[primary_key]
    #[auto_inc]
    pub request_id: u64,
    pub party_id: u64,
    pub kind: PartyRequestKind,
    pub initiator: Identity,
    pub recipient: Identity,   // whoever must /party accept or /party decline
    pub created_at: Timestamp,
}
```

`PartyMemberRow` being keyed by `identity` as its primary key is what enforces "one party per player" — no separate uniqueness check needed, and it gives an O(1) lookup for the friendly-fire guard in `apply_damage`.

A new `crates/stdb-module/src/reducers/parties.rs` holds one reducer per verb (`party_invite`, `party_join`, `party_accept`, `party_decline`, `party_leave`), registered in `reducers/mod.rs` next to `chat.rs`. `/party list` and bare `/party` need **no reducer**: party membership is a `public` subscribed table, so the client already has the caller's party roster locally and can render it (and the help text) purely client-side, the same way any other subscribed table is rendered today.

On the client, `crates/presentation/src/ui/chat.rs`'s existing submit handler gets a `/party ` prefix check (added, not rebuilt) that parses the subcommand and calls the matching reducer instead of `send_chat_message`; anything not starting with `/party ` (or `/party` alone) continues through the existing chat path unchanged. Invite/request notifications go out through `PlayerMessageEvent` exactly like `respawn` does today, so they render in the existing chat + notice UI with no new message type required for v1.

## Decisions

Confirmed with the user (2026-08-17):

1. Party commands are implemented as **dedicated reducers per verb** (`party_invite`, `party_join`, `party_accept`, `party_decline`, `party_leave`), independent of `plans/account-chat-admin.md` Slice 5's not-yet-built generic `/`-command dispatcher. If that dispatcher is built later, party commands can be wired into it, but this plan does not block on it.
2. Maximum party size is **5**.

Additional default decisions made to keep the command surface exactly as specified (7 commands, no extras) — flag for amendment if any of these don't match intent:

3. There is no separate "create party" command. `/party invite <name>` implicitly creates a new party (with the sender as leader) the first time a player who isn't already in one invites someone.
4. `/party invite <name>` may only be issued by the party's current leader (or by a player not yet in any party, per Decision 3). A non-leader member attempting to invite is rejected.
5. `/party join <name>` names the party's **leader**; it creates a `JoinRequest` addressed to that leader. The leader accepts/declines it with `/party accept <name>` / `/party decline <name>`, exactly like an invite is accepted/declined by the invitee — `accept`/`decline` always resolve against whichever pending request (either direction) exists between the sender and `<name>`.
6. A recipient (invitee or leader) can have **at most one pending party request at a time**; a second request while one is outstanding is rejected with feedback, not queued.
7. There is no `/party kick`. To remove a member, that member must `/party leave`. This matches the requested command list exactly; kicking is out of scope for this plan.
8. If the leader leaves a party with other members remaining, leadership transfers to the longest-tenured remaining member (`joined_at` ascending). If leaving empties the party, the `PartyRow` and any of its pending `PartyRequestRow`s are deleted (the party disbands).
9. Friendly fire prevention applies to **damage only**; it does not restrict beneficial spells/abilities cast on party members (out of scope — no such interactions exist in `apply_damage` today anyway).
10. Party membership follows `Player`'s current `Identity` key, consistent with the rest of the codebase pre-account-system; no special handling is needed for `plans/account-chat-admin.md`'s later account layer since both plans predate that plan's database wipe.

## Global acceptance criteria

- A player can only be a member of one party at a time.
- `/party invite`, `/party join`, `/party accept`, `/party decline`, `/party leave` never leave the `party`/`party_member`/`party_request` tables in an inconsistent state (e.g. a member row pointing at a deleted party, or a lingering request after both sides are already grouped).
- Two members of the same party can never damage each other through `apply_damage`, regardless of the source (spell, DoT, or future PvP path), while damage against non-party entities (enemies, bosses, dummies, other players outside the party) is unaffected.
- `/party list` and bare `/party` never call a reducer; they render from already-subscribed client state.
- Invalid usage (unknown target, self-invite, inviting/joining while already grouped, exceeding the size-5 cap, accepting/declining a non-existent request) produces no state change and clear feedback to the sender only.
- Party actions never affect players outside the two identities directly involved (or, for `leave`, the sender's own party).

## Vertical slices

Every slice follows **RED → GREEN → MUTATE → KILL MUTANTS → REFACTOR**. Before implementation of each slice, load the `tdd`, `testing`, `mutation-testing`, and `refactoring` skills. Also load `rust-guidelines` for Rust work and `modern-web-guidance` if any frontend surface is touched (none is expected — this plan is Bevy client + `stdb-module` only).

### Slice 1: Form, join, and manage a party through the chat input

**Value**: A player can invite, request to join, accept, decline, leave, and list a party entirely through the existing chat box, with no server-side friendly-fire guarantee yet (that's Slice 2).

**Production path**: Chat input `/party <verb> [name]` → client-side prefix parser in `crates/presentation/src/ui/chat.rs` → matching reducer in the new `crates/stdb-module/src/reducers/parties.rs` → `party`/`party_member`/`party_request` table mutation → `PlayerMessageEvent` feedback to the relevant identities → client renders via the existing chat/notice UI. `/party list` and bare `/party` render directly from subscribed `party_member` rows client-side, no reducer call.

**Acceptance criteria**:

- `party_invite`: creates a party if the sender has none (Decision 3); rejects non-leader senders (Decision 4), self-invites, inviting an already-grouped target, inviting while the party is at the size-5 cap, and inviting a target with an existing pending request (Decision 6).
- `party_join`: creates a `JoinRequest` addressed to the named leader; rejects joining while already in a party, an unknown/non-leader name, or a full party.
- `party_accept` / `party_decline`: resolve against the correct pending request regardless of its direction (Decision 5); `accept` inserts the `party_member` row (creating the party for a fresh invite chain, or attaching to the existing one) and deletes the request; `decline` only deletes the request. Both reject when no matching pending request exists.
- `party_leave`: removes the sender's `party_member` row; promotes the next leader or disbands per Decision 8; rejects when the sender isn't in a party.
- `/party` alone and `/party list` never reach the server; they render local state, including an empty-party case ("You are not in a party").
- Ordinary chat text (no `/party` prefix) and any other existing `/`-looking text continue through `send_chat_message` unchanged (regression coverage per `plans/account-chat-admin.md` Slice 4's existing-behavior concern).

**RED**: Add reducer tests in `parties.rs` for each verb's happy path and the rejection cases above, plus a client-side parser test in `chat.rs` for prefix detection, argument parsing, and the "unrecognized `/party` subcommand shows help" case. Cover likely mutants around: request-direction resolution in `accept`/`decline`, the size-5 boundary (exactly 5 vs 6th member), leader-only invite enforcement, and leader-promotion tie-breaking on `leave`.

**GREEN**: Add the three tables, `parties.rs` reducers, `reducers/mod.rs` registration, and the chat-input prefix branch. Keep `send_chat_message` untouched.

**MUTATE**: Run mutation testing over `parties.rs` and the new chat-parser branch.

**KILL MUTANTS**: Strengthen boundary tests (cap, single-pending-request, direction resolution) until surviving mutants are gone; verify every rejection path leaves the tables unchanged.

**REFACTOR**: Only after all verbs work end-to-end; do not introduce a generic command-AST abstraction here (that belongs to `plans/account-chat-admin.md` Slice 5 if it happens later, per Decision 1).

### Slice 2: Prevent friendly-fire damage between party members

**Value**: The core guarantee requested — two players in the same party cannot damage each other, no matter the damage source, now or as future PvP paths are added.

**Production path**: `apply_damage(ctx, target, source, amount)` → resolve `target`'s owning `Identity` via `Player.entity_id` (unique lookup) → if `target` isn't player-owned, proceed unchanged (fast path, no party lookup) → otherwise resolve `source`'s owning `Identity` the same way → if both resolve and `party_member` places them in the same `party_id`, skip the damage entirely (no health change, no `DamageEventRow`, no death) and optionally notify the attacker ("You cannot attack a party member.") → otherwise proceed with existing mitigation/health/death logic unchanged.

**Acceptance criteria**:

- Damage between two members of the same party is always blocked, regardless of call site (`effects.rs` spell damage, `combat.rs` periodic/DoT damage).
- Damage against non-player entities (enemies, bosses, dummies, NPCs) is completely unaffected — the guard must not add a party lookup on that path (`EntityKindRow` check first).
- Damage between two players who are **not** in the same party (including two players each in their own, different party) is unaffected.
- Damage from a source with no owning identity (AI, environmental) is unaffected.
- A blocked hit does not produce a `DamageEventRow`, does not flag death, and does not desync health on either side.

**RED**: Add tests in `sim/combat.rs` covering: same-party player-vs-player (blocked), different-party player-vs-player (allowed), player-vs-enemy (allowed, and ideally shown not to perform a party lookup at all), self-damage/no-source (unaffected), and a party member damaging an enemy while another human player is nearby but not grouped (allowed). Cover mutants that invert the party-equality check or drop the `EntityKindRow` fast-path guard (which would be a correctness regression, not just a perf one, if it changed lookup order incorrectly).

**GREEN**: Add the identity-resolution helper (`Identity` from `entity_id` via `Player`'s unique index) and the guard at the top of `apply_damage`, before mitigation math.

**MUTATE**: Run mutation testing over `apply_damage` and the new guard/helper.

**KILL MUTANTS**: Add tests for the exact same-party boundary (both members, one leaves mid-fight — no test should assume static membership) and confirm no dead code path still allows the damage through a different call site.

**REFACTOR**: Extract the identity-resolution helper if it's reused by more than the party guard; do not preemptively generalize into a full PvP-rules engine.

## Pre-PR quality gate

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cd crates/stdb-module && spacetime build`
4. Mutation testing for the current slice
5. Manual verification with two identities: invite → accept → both in `party_member`; attempt to damage each other in-game and confirm it's blocked while damaging an enemy still works
6. Confirm `git diff` contains no unrelated pre-existing local changes

## Security requirements

- Every `parties.rs` reducer validates the sender's own membership/request state server-side; the client-sent name is only ever used to resolve a target `Player`, never trusted for authorization.
- No reducer allows a player to act on another player's membership except through the mutual invite/request + accept/decline flow (no reducer lets A unilaterally place B into or out of a party).
- The friendly-fire guard fails closed on ambiguous data: if identity resolution for `target` or `source` is inconclusive, damage proceeds through the existing (unchanged) path rather than being silently skipped — this plan must not introduce a way to make a target invulnerable by accident.

---

Decisions 1–2 were confirmed directly with the user on 2026-08-17; decisions 3–10 are proposed defaults chosen to match the exact command list requested and the codebase's existing conventions — flag any of them for the user to amend before starting Slice 1 if they don't match intent.
