//! One live SpacetimeDB connection, wrapped for use from axum handlers.
//!
//! # Why a real connection, not a one-shot HTTP call
//!
//! SpacetimeDB's `POST /v1/database/<name>/call/<reducer>` looks like it
//! could authenticate a web session statelessly: pass a bearer token, get a
//! result. It cannot, for this module specifically. Verified empirically
//! against the local server: SpacetimeDB treats *every* HTTP call as its own
//! connection — `client_connected` fires before the reducer runs and
//! `client_disconnected` fires immediately after, even when the same
//! Identity token is reused across calls. `client_disconnected`
//! (`reducers::lifecycle::client_disconnected`) unconditionally deletes the
//! caller's `Session` row, so a `Session` created by `register`/`login` over
//! HTTP is already gone before the HTTP response comes back — reusing the
//! same token does not help, because it is the *connection*, not the
//! Identity, that `Session`'s lifetime is tied to.
//!
//! So a web session needs the same thing a game session has: a connection
//! that stays open. This type holds one, driven by a background task calling
//! [`spacetimedb_sdk`]'s `run_async` for as long as the session lives — see
//! `super::session` for how long that is and how it is torn down.

use std::sync::{Arc, Mutex};

use spacetimedb_sdk::{DbContext, Table};
use tokio::sync::oneshot;

use super::module_bindings::login_reducer::login;
use super::module_bindings::logout_reducer::logout;
use super::module_bindings::register_reducer::register;
use super::module_bindings::{
    DbConnection, ErrorContext, Player, PlayerTableAccess, ReducerEventContext, SessionTableAccess,
};

/// One of the caller's own characters, for the `/profile` endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CharacterSummary {
    pub character_id: u64,
    pub display_name: String,
    pub online: bool,
}

/// What a `*_then` callback is handed: the module's own `Result`, or the SDK
/// failing to decode one. Mirrors `bevymmo_client::stdb::plugin`'s type of
/// the same name — same SDK, same shape.
type ReducerOutcome = Result<Result<(), String>, spacetimedb_sdk::__codegen::InternalError>;

/// Turns a reducer's `_then` callback into something `await`-able: sends the
/// outcome down `tx` instead of requiring the caller to register their own
/// callback and poll for it.
fn outcome_sender(
    tx: oneshot::Sender<Result<(), String>>,
) -> impl FnOnce(&ReducerEventContext, ReducerOutcome) + Send + 'static {
    move |_ctx, outcome| {
        let result = match outcome {
            Ok(Ok(())) => Ok(()),
            Ok(Err(reason)) => Err(reason),
            Err(err) => Err(err.to_string()),
        };
        // The receiver may already be gone if the HTTP request that started
        // this call timed out or the client disconnected; nothing to do.
        let _ = tx.send(result);
    }
}

/// A live SpacetimeDB connection plus the background task advancing it.
/// Cloning shares the same underlying connection — cheap, and what the
/// session store wants: every request for the same browser session reuses
/// one connection rather than opening a new one.
#[derive(Clone)]
pub struct GatewayConnection {
    conn: Arc<DbConnection>,
}

impl GatewayConnection {
    /// Opens a fresh connection, subscribes to the tables this gateway
    /// needs, and spawns the background task that keeps it alive. Returns
    /// once the initial subscription has applied, so callers can read
    /// `player`/`session` state immediately after this returns.
    pub async fn connect(uri: &str, module: &str) -> Result<Self, String> {
        let conn = DbConnection::builder()
            .with_uri(uri)
            .with_database_name(module)
            .on_connect_error(|_ctx: &ErrorContext, err| {
                tracing::error!("gateway SpacetimeDB connection error: {err}");
            })
            .on_disconnect(|_ctx, err| match err {
                Some(err) => tracing::warn!("gateway SpacetimeDB connection dropped: {err}"),
                None => tracing::debug!("gateway SpacetimeDB connection closed"),
            })
            .build()
            .map_err(|err| err.to_string())?;
        let conn = Arc::new(conn);

        let run_conn = Arc::clone(&conn);
        tokio::spawn(async move {
            if let Err(err) = run_conn.run_async().await {
                tracing::warn!("gateway SpacetimeDB connection ended: {err}");
            }
        });

        let this = Self { conn };
        this.subscribe().await?;
        Ok(this)
    }

    /// Subscribes to `player` (for the character roster) and `session` (for
    /// this connection's own `account_id` — see `tables::Session`'s doc
    /// comment on why it is public). Awaits the initial snapshot so the
    /// caller does not race an empty local cache.
    async fn subscribe(&self) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        let tx = Arc::new(Mutex::new(Some(tx)));

        let applied_tx = Arc::clone(&tx);
        let error_tx = Arc::clone(&tx);
        self.conn
            .subscription_builder()
            .on_applied(move |_ctx| {
                if let Some(tx) = applied_tx.lock().unwrap().take() {
                    let _ = tx.send(Ok(()));
                }
            })
            .on_error(move |_ctx, err| {
                if let Some(tx) = error_tx.lock().unwrap().take() {
                    let _ = tx.send(Err(err.to_string()));
                }
            })
            .subscribe(["SELECT * FROM player", "SELECT * FROM session"]);

        rx.await
            .map_err(|_| "subscription dropped before it applied".to_string())?
    }

    /// This connection's own SpacetimeDB `Identity`, once the handshake has
    /// completed (always true by the time [`Self::connect`] returns).
    fn identity(&self) -> Option<spacetimedb_sdk::Identity> {
        self.conn.try_identity()
    }

    /// The `account_id` this connection authenticated as, if `register`/
    /// `login` has succeeded on it. Resolved from the (public) `session`
    /// table rather than tracked locally, so it is always consistent with
    /// what the server actually recorded.
    pub fn account_id(&self) -> Option<u64> {
        let identity = self.identity()?;
        self.conn
            .db()
            .session()
            .iter()
            .find(|row| row.identity == identity)
            .map(|row| row.account_id)
    }

    /// This account's own characters (up to
    /// `bevymmo_module::MAX_CHARACTERS_PER_ACCOUNT`), from the already
    /// public `player` table filtered to this connection's `account_id`.
    /// `None` if this connection has not authenticated yet.
    pub fn characters(&self) -> Option<Vec<CharacterSummary>> {
        let account_id = self.account_id()?;
        Some(
            self.conn
                .db()
                .player()
                .iter()
                .filter(|row| row.account_id == account_id)
                .map(character_summary)
                .collect(),
        )
    }

    pub async fn register(&self, email: String, password: String) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.conn
            .reducers()
            .register_then(email, password, outcome_sender(tx))
            .map_err(|err| err.to_string())?;
        rx.await
            .map_err(|_| "connection closed before the server replied".to_string())?
    }

    pub async fn login(&self, email: String, password: String) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.conn
            .reducers()
            .login_then(email, password, outcome_sender(tx))
            .map_err(|err| err.to_string())?;
        rx.await
            .map_err(|_| "connection closed before the server replied".to_string())?
    }

    /// Ends this connection's authenticated session server-side (deletes its
    /// `Session` row) without closing the socket. A no-op if it was never
    /// authenticated. The gateway calls this and then drops the connection
    /// entirely — see `super::session::SessionStore::end`.
    pub async fn logout(&self) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.conn
            .reducers()
            .logout_then(outcome_sender(tx))
            .map_err(|err| err.to_string())?;
        rx.await
            .map_err(|_| "connection closed before the server replied".to_string())?
    }

    /// Closes the underlying socket. `client_disconnected` on the module
    /// then deletes this connection's `Session` row server-side, same as an
    /// explicit [`Self::logout`] — see `reducers::lifecycle::client_disconnected`.
    pub fn disconnect(&self) {
        if let Err(err) = self.conn.disconnect() {
            tracing::warn!("failed to queue gateway SpacetimeDB disconnect: {err}");
        }
    }
}

fn character_summary(row: Player) -> CharacterSummary {
    CharacterSummary {
        character_id: row.character_id,
        display_name: row.display_name,
        online: row.online,
    }
}
