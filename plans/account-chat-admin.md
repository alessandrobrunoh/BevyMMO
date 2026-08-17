# Plan: Accounts, Chat, and Admin Commands

**Branch**: TBD  
**Status**: Proposed

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
-
 Existing local changes must not be overwritten.

## Proposed architecture

Keep SpacetimeDB authoritative for accounts, roles, identity bindings, characters, messages, command effects, and audit records. The gateway should be an HTTP facade for the frontend, not a second gameplay server.

Use the SpacetimeDB `Identity` as the authenticated connection identity and add an application-level account binding (`Identity -> Account`). Passwords must only be stored as salted slow password hashes, never plaintext and never in logs. The hash algorithm must be verified for WASM compatibility before finalizing the schema.

The Bevy client should expose explicit states such as `logged_out`, `authenticating`, `authenticated`, and `rejected`; a cached SpacetimeDB token must not be treated as the complete user-facing login model.

## Decisions required before implementation

1. Does “list of my accounts” mean a list of playable characters, or can one user own multiple separate game accounts?
2. Must the Bevy client ask for username/password before entering the world?
3. Can password recovery be deferred from the first version?
4. Are the initial roles only `player` and `admin`, or are `moder
ator`/`gm` also required?
5. Should the first chat version support only global chat, or also whispers/party/guild channels?
6. Which commands should ship first? Proposed: `notify`, then `give_item`; `teleport` and `reseed` should be added only after their permission boundaries are explicit.
7. Should the frontend profile show playable characters, connected devices/sessions, or both?

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

**Value**: A player gets a permanent username/password account while retaining the secure SpacetimeDB connection identity.

**Production path**: Bevy login screen → register/login reducer → persistent account and identity binding → character load/create → success or rejection feedback.

**Acceptance criteria**:

- Username length and normalization rules are enforced and uniqueness is case-insensitive.
- Password validation and salted hash verification are enforced server-side.
- Failed login does not reveal whether a username exists.
- A new connection authenticated as the same account loads the same profile.
- `join` cannot create a character for an unauthenticated account.

**RED**: Add domain tests for normalization/validation, module tests for register/login/binding, and client tests for authentication states. Cover likely mutants involving username comparison, incorrect passwords, identity binding, and duplicate registration.

**GREEN**: Add the minimal account schema, WASM-compatible password hashing, atomic reducers, and Bevy authentication state. Keep the existing GM bootstrap separate.

**MUTATE**: Run mutation testing over validation and authentication tests.

**KILL MUTANTS**: Strengthen boundary tests and verify failed authentication has no state-changing side effects.

**REFACTOR**: Only after the real path works; do not introduce a speculative auth-provider abstraction.

### Slice 2: Authenticate from the frontend and show the real profile

**Value**: The same account works on the website and the user can see their own characters/accounts instead of mock data.

**Production path**: Angular form → authenticated gateway endpoint → filtered account/character query → `/profile` → logout/session expiry handling.

**Acceptance criteria**:

- The real auth service replaces `AuthMockService` for login/register.
- Session storage and expiry behavior are explicit.
- `/profile` returns only records belonging to the authenticated user.
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

### Slice 4: Add the bottom-left global chat UI and system notifications

**Value**: Every player can communicate through a familiar in-game chat control, and gameplay systems can show targeted feedback.

**Production path**: Bottom-left chat widget → click or press **Enter** to focus the input → submit message → `send_chat_message` reducer → validated global event → every subscribed client renders `$name: $message`. Gameplay reducers emit targeted system notifications through the same notification path.

**UI behavior**:

- The chat is visible in the bottom-left of the game screen.
- Clicking the chat area focuses the text input.
- Pressing **Enter** focuses the input when chat is not already focused.
- Pressing **Enter** while the input is focused submits the message.
- Empty messages are ignored or rejected without a server call.
- The global chat format is exactly `$name: $message`.
- Chat input must not steal movement/gameplay keyboard controls while it is unfocused.
- Escape or an explicit focus-loss action returns keyboard input to gameplay.

**Acceptance criteria**:

- A message sent by one connected player is visible to all connected players.
- The rendered message contains the server-provided display name and validated text in `$name: $message` format.
- Empty, oversized, and rate-limited messages are rejected safely.
- Targeted notifications are delivered only to the intended player.
- Offline recipients do not cause transaction failures or panics.
- Chat and system notifications are visually distinct and do not render unsafe HTML/script.
- Enter/focus behavior is covered without breaking movement or existing shortcuts.

**RED/GREEN/MUTATE/KILL MUTANTS**: Test reducer validation and broadcast behavior, then test the Bevy UI focus/submission state. Cover target isolation, length limits, rate limiting, Enter while focused/unfocused, empty submission, and input leakage into gameplay. Add an inventory notification such as “Quest item added to your inventory.”

### Slice 5: Add optional admin commands through the same chat input

**Value**: An admin can use the same bottom-left UI for debugging without exposing an arbitrary shell, SQL executor, or code evaluator; normal players continue to use it as global chat.

**Production path**: Bottom-left input → command prefix/syntax (to be confirmed) → typed parser/command AST → server dispatcher → role policy → game effect → audit + chat feedback. Ordinary text continues through the global chat path.

**Acceptance criteria**:

- Normal text is always treated as global chat.
- Only an explicit command syntax is treated as an admin command.
- Only authorized admins can invoke the dispatcher.
- Invalid commands have no side effects and return safe usage feedback.
- `notify <player> <text>` delivers a targeted notification.
- `give_item` validates item id and quantity and updates inventory atomically.
- Each command records actor, normalized command, result, and timestamp without passwords or tokens.
- Command responses are visible in the chat UI with a distinct system/admin style.

**RED/GREEN/MUTATE/KILL MUTANTS**: Test parser, allowlist, authorization, target lookup, quantity bounds, retry behavior, transaction effects, and the distinction between normal chat and commands. The client sends structured command data; it never sends Rust code to be evaluated server-side.

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

---

This plan remains **Proposed** until the open decisions and Slice 1 acceptance criteria are confirmed.
