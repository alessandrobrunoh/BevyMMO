//! In-memory store mapping a gateway session cookie to its live
//! [`GatewayConnection`].
//!
//! The cookie value is an opaque id the gateway mints itself — never a
//! SpacetimeDB Identity or token, both of which stay server-side inside the
//! connection this id looks up. A leaked cookie only grants whatever this
//! particular session already had; it grants no direct SpacetimeDB access.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use uuid::Uuid;

use super::connection::GatewayConnection;

/// How long an idle web session's SpacetimeDB connection stays open.
/// Comfortably longer than a normal browsing session, short enough that a
/// forgotten browser tab does not hold a connection open forever.
const SESSION_IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// How often the reaper checks for idle sessions. Independent of the
/// timeout itself: this is a polling interval, not a deadline.
const REAP_INTERVAL: Duration = Duration::from_secs(60);

pub type SessionId = String;

struct Entry {
    connection: GatewayConnection,
    last_seen: Instant,
}

/// Cloning shares the same underlying map (via `Arc`) — cheap, and what
/// letting every axum handler hold its own copy of the state wants.
#[derive(Clone, Default)]
pub struct SessionStore {
    entries: Arc<RwLock<HashMap<SessionId, Entry>>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mints a new session id for an already-open, already-authenticated
    /// `connection`. Deliberately takes a connection rather than opening one
    /// itself: `register`/`login` must succeed *before* a cookie is issued,
    /// so a failed login attempt never hands the browser a session id for a
    /// connection nobody can use.
    pub async fn create(&self, connection: GatewayConnection) -> SessionId {
        let id = Uuid::new_v4().to_string();
        self.insert(id.clone(), connection).await;
        id
    }

    /// Stores `connection` under a caller-chosen `id`. Cookie sessions use a
    /// random UUID from [`Self::create`]; API-key sessions reuse this with a
    /// stable `ak:{sha256}` id so later Bearer requests reuse the socket.
    pub async fn insert(&self, id: SessionId, connection: GatewayConnection) {
        self.entries.write().await.insert(
            id,
            Entry {
                connection,
                last_seen: Instant::now(),
            },
        );
    }

    /// The connection for `id`, refreshing its idle timer. `None` if `id` is
    /// unknown or was already reaped for inactivity.
    pub async fn get(&self, id: &str) -> Option<GatewayConnection> {
        let mut entries = self.entries.write().await;
        let entry = entries.get_mut(id)?;
        entry.last_seen = Instant::now();
        Some(entry.connection.clone())
    }

    /// Ends the session: best-effort `logout` on its connection (clears the
    /// server-side `Session` row explicitly rather than waiting for the
    /// socket close to do it), then drops it from the store and disconnects.
    pub async fn end(&self, id: &str) {
        if let Some(entry) = self.entries.write().await.remove(id) {
            let _ = entry.connection.logout().await;
            entry.connection.disconnect();
        }
    }

    /// Closes and drops every connection idle longer than
    /// [`SESSION_IDLE_TIMEOUT`].
    async fn reap_idle(&self) {
        let expired: Vec<SessionId> = {
            let entries = self.entries.read().await;
            entries
                .iter()
                .filter(|(_, entry)| entry.last_seen.elapsed() > SESSION_IDLE_TIMEOUT)
                .map(|(id, _)| id.clone())
                .collect()
        };
        for id in expired {
            tracing::debug!(session = %id, "gateway session idle timeout");
            self.end(&id).await;
        }
    }

    /// Spawns the periodic idle-session reaper. Call once, from `main`.
    pub fn spawn_reaper(&self) {
        let store = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(REAP_INTERVAL);
            loop {
                interval.tick().await;
                store.reap_idle().await;
            }
        });
    }
}
