# Plan: Accounts, Chat, and Admin Commands

**Branch**: TBD  
**Status**: Decisions confirmed (2026-08-17) — ready to start Slice 1

## Goal

Allow a player to create a permanent username/password account, use it from both the Bevy client and Angular frontend, view their characters/accounts in a profile, exchange chat messages, and allow authorized admins to execute safe server-side commands.

## Current architecture and constraints

- `crates/stdb-module` is the authoritative game server; the gateway and Bevy client are not authoritative.
- `Player` is currently keyed by `spacetimedb::Identity`.
- The Bevy client caches the SpacetimeDB token locally.
- The frontend login is still a mock: `apps/frontend/src/app/core/services/auth-mock.service.ts`.
- `apps/gateway` currently only exposes health/welcome endpoints.
- `player_message` already exists as a transient SpacetimeDB event for one-shot player notifications, but it is not a chat history.
- GM authorization currently uses the compile-time `BEVYMMO_GM_IDENTITIES` allowlist.
- The bottom-left chat UI (click/Enter/Escape focus behavior) and the `send_chat_message` reducer (`$name: $message` formatting, length/content validation) already exist and work end-to-end (`crates/presentation/src/ui/chat.rs`, `crates/stdb-module/src/reducers/chat.rs`). Slice 4 only needs to add server-side rate limiting and regression coverage, not rebuild the feature.
- The current SpacetimeDB database will be wiped immediately before Slice 1 ships; no migration path for existing identities/characters is needed.
- Existing local changes must not be overwritten.

## Proposed architecture

Keep SpacetimeDB authoritative for accounts, roles, identity bindings, characters, messages, command effects, and audit records. The gateway should be an HTTP facade for the frontend, not a second gameplay server.

Use the SpacetimeDB `Identity` as the authenticated connection identity and add an application-level account binding (`Identity -> Account`). Passwords must only be stored as salted slow password hashes, never plaintext and never in logs. The hash algorithm must be verified for WASM compatibility before finalizing the schema.

The Bevy client should expose explicit states such as `logged_out`, `authenticating`, `authenticated`, and `rejected`; a cached SpacetimeDB token must not be treated as the complete user-facing login model.

## Decisions (confirmed 2026-08-17)

1. One account (email + password) can own up to 3 characters (e.g. `Galvdon1`, `Galvdon2`, `Galvdon3`). "List of my accounts" in the profile means these characters, not multiple separate game accounts.
2. The Bevy client must authenticate with email + password before entering the world; no guest/anonymous entry.
3. Password recovery is deferred; out of scope for this plan.
4. Initial roles are only `player` and `admin`.
5. First chat version is global only; party/whisper channels are a later iteration.
6. First admin commands: `/kill <player>` (deals lethal damage through the normal combat/health path, triggering death → respawn; must never delete the character or account) and `/give <player> <item_name>`.
7. The frontend profile shows both playable characters and connected sessions/devices.
8. Chat rate limiting is a per-account token bucket (burst allowance, then a fixed refill rate) evaluated with `ctx.timestamp()`, not per-connection — reconnecting must not reset the limit.
9. Command syntax uses a `/` prefix (e.g. `/kill`, `/give`); anything else is treated as normal global chat text.
10. The existing SpacetimeDB database will be wiped immediately before Slice 1 ships. No migration path for pre-existing identities/characters is needed or should be built.
11. The frontend needs both a login page and a dedicated register page, not login only.

## Global acceptance criteria

- A new user can create an account with a normalized, unique, permanent username and a validated password.
- An existing user can authenticate from both Bevy and the frontend without accidentally creating a second profile.
- Passwords are never returned to clients, stored plaintext, or included in logs/errors.
- The profile only exposes accounts/characters owned by the authenticated user.
- A non-admin cannot execute admin commands, including through direct reducer calls.
- Global chat messages include author, timestamp, and validated content.
- System messages can be delivered to a specific player, for example: “Quest item added to your inventory.”
- Invalid commands do not change state; valid admin commands produce a verifiable effect, feedback, and audit entry.
- Authentication, authorization, rate-limit, and validation failures are visible to clients without leaking sensitive details.

## Vertical slices

Every slice follows **RED → GREEN → MUTATE → KILL MUTANTS → REFACTOR**. Before implementation of each slice, load the `tdd`, `testing`, `mutation-testing`, and `refactoring` skills. Also load `rust-guidelines` for Rust work and `modern-web-guidance` for frontend work.

### Slice 1: Register and authenticate an account from the game client

**Value**: A player gets a permanent email/password account while retaining the secure SpacetimeDB connection identity, and can create up to 3 characters under it.

**Production path**: Existing SpacetimeDB database is wiped (Decision 10) → Bevy login screen (email + password, register or login) → register/login reducer → persistent account and identity binding → character list/create (max 3 per account) → success or rejection feedback.

**Acceptance criteria**:

- Email format and normalization rules are enforced and uniqueness is case-insensitive.
- Password validation and salted hash verification are enforced server-side.
- Failed login does not reveal whether an email is registered.
- A new connection authenticated as the same account loads the same profile and character list.
- An account cannot own more than 3 characters; creating a 4th is rejected server-side.
- `join`/character creation cannot happen for an unauthenticated account.

**RED**: Add domain tests for normalization/validation, module tests for register/login/binding, and client tests for authentication states. Cover likely mutants involving email comparison, incorrect passwords, identity binding, duplicate registration, and the 3-character cap (off-by-one at the boundary).

**GREEN**: Add the minimal account schema, WASM-compatible password hashing, atomic reducers, character-cap enforcement, and Bevy authentication state. Keep the existing GM bootstrap separate.

**MUTATE**: Run mutation testing over validation and authentication tests.

**KILL MUTANTS**: Strengthen boundary tests and verify failed authentication has no state-changing side effects.

**REFACTOR**: Only after the real path works; do not introduce a speculative auth-provider abstraction.

### Slice 2: Authenticate from the frontend and show the real profile

**Value**: The same account works on the website and the user can see their own characters/accounts instead of mock data.

**Production path**: Angular register page + login page (email + password) → authenticated gateway endpoint → filtered account/character/session query → `/profile` (characters + connected sessions) → logout/session expiry handling.

**Acceptance criteria**:

- The real auth service replaces `AuthMockService` for both login and register.
- A dedicated `/register` page exists alongside `/login`.
- Session storage and expiry behavior are explicit.
- `/profile` returns only characters and sessions belonging to the authenticated user.
- Anonymous access redirects to `/login`.
- Loading, validation, server errors, and logout are covered by tests.

**RED**: Add gateway tests for status codes and ownership filtering, plus Angular tests for form submission, guard, profile rendering, loading, and expiry. Cover mutants that remove ownership filtering or accept expired sessions.

**GREEN**: Implement the minimal gateway auth adapter and profile route without changing unrelated static website content.

**MUTATE**: Run mutation testing over gateway authorization and frontend auth/guard logic.

**KILL MUTANTS**: Add tests for cross-user access, invalid sessions, and logout.

### Slice 3: Replace static GM allowlists with persistent roles

**Value**: Admin permissions are persistent, explicit, and enforced consistently by the server.

**Production path**: Authenticated account → persistent role → centralized permission policy → protected reducer → audit/error.

**Acceptance criteria**:

- A default `player` cannot call admin reducers.
- An `admin` can call only commands allowed by the role policy.
- Role assignment is itself protected and audited.
- Authorization never relies on display names or client-provided role values.
- Existing GM reducers (`gm_set_prop_override`, `gm_clear_prop_override`, `gm_reseed_world`) have regression coverage.

**RED/GREEN/MUTATE/KILL MUTANTS**: Test player/admin identities, missing bindings, unknown roles, inverted permission checks, and the temporary `BEVYMMO_GM_IDENTITIES` bootstrap path.

### Slice 4: Chat rate limiting and regression coverage (UI and broadcast already exist)

**Value**: The bottom-left chat widget, focus/submit/escape behavior, and the `send_chat_message` reducer (with `$name: $message` formatting and content validation) already exist in `crates/presentation/src/ui/chat.rs` and `crates/stdb-module/src/reducers/chat.rs`. This slice closes the remaining gap — server-side rate limiting — and locks the existing behavior down with regression tests instead of rebuilding it.

**Production path**: `send_chat_message` reducer → per-account token bucket check (Decision 8) → validated global event → every subscribed client renders `$name: $message`. Gameplay reducers continue emitting targeted system notifications through the existing `player_message` path.

**Already implemented (verify with regression tests, do not rebuild)**:

- Bottom-left widget, click-to-focus, Enter-to-focus-when-unfocused, Enter-to-submit-when-focused, Escape-to-release-focus.
- Empty-message client-side guard before any server call.
- `$name: $message` formatting resolved server-side from the caller's display name.
- Visual distinction between chat lines and system notifications (`ui/notices/systems.rs`).

**New acceptance criteria for this slice**:

- A per-account token bucket rejects messages sent faster than the agreed burst/refill rate, without a server-side panic or transaction failure.
- The rate-limit rejection is surfaced to the sender without affecting other players.
- Reconnecting does not reset the rate-limit counter.
- Targeted notifications continue to be delivered only to the intended player, including when the recipient is offline.

**RED/GREEN/MUTATE/KILL MUTANTS**: Add reducer tests for the token-bucket boundary (exactly at limit, one over, after the refill window elapses), plus regression tests for the existing focus/submit/escape/format behavior so the working UI can't silently break while this slice touches the reducer.

### Slice 5: Add admin commands through the same chat input

**Value**: An admin can use the same bottom-left UI for debugging without exposing an arbitrary shell, SQL executor, or code evaluator; normal players continue to use it as global chat.

**Production path**: Bottom-left input → `/`-prefixed command syntax (Decision 6/9) → typed parser/command AST → server dispatcher → role policy → game effect → audit + chat feedback. Ordinary text (no `/` prefix) continues through the global chat path unchanged.

**Acceptance criteria**:

- Normal text is always treated as global chat; only text starting with `/` is parsed as a command.
- Only authorized admins can invoke the dispatcher; a non-admin sending `/kill ...` is rejected with no side effects, not silently posted as chat.
- Invalid commands (unknown name, wrong arg count/type, unknown target) have no side effects and return safe usage feedback.
- `/kill <player>` deals lethal damage through the existing combat/health path, triggering the normal death → respawn flow; it must never delete or otherwise destroy the target's character or account.
- `/give <player> <item_name>` validates the item exists and the target exists, then updates inventory atomically.
- Each command records actor, normalized command, target, result, and timestamp — never passwords or tokens.
- Command responses are visible in the chat UI with a distinct system/admin style.

**RED/GREEN/MUTATE/KILL MUTANTS**: Test parser (prefix detection, malformed args), the admin allowlist, target lookup (missing/offline player), `/kill` routing through the real damage/respawn path (not a separate death path), `/give` item/quantity validation, and the boundary between normal chat and command parsing. The client sends structured command data; it never sends Rust code to be evaluated server-side.

## Pre-PR quality gate

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cd crates/stdb-module && spacetime build`
4. `cd apps/frontend && npm test -- --run`
5. `cd apps/frontend && npm run build`
6. Mutation testing for the current slice
7. Manual verification with two identities: profile isolation, targeted chat isolation, and admin permissions
8. Confirm `git diff` contains no unrelated pre-existing local changes

## Security requirements

- Never store plaintext passwords or use a fast unsalted hash as the password storage format.
- Never expose password hashes, salts, internal bindings, or unfiltered player tables to the frontend.
- Admin commands must be a typed allowlist; never `eval`, arbitrary shell, arbitrary SQL, or executable code from the client.
- Chat requires content length limits, rate limiting, and safe rendering.
- Chat rate limiting is per-account (Decision 8), not per-connection, so it cannot be bypassed by reconnecting.

---

All open decisions are confirmed as of 2026-08-17 (see "Decisions" above). This plan is ready to start Slice 1.
