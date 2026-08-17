//! Shared, read-only view of the public `player` table backing
//! `/public/accounts/*`.
//!
//! One SpacetimeDB connection is held open for the gateway's whole lifetime and
//! reused by every unauthenticated request, rather than one connection per HTTP
//! call: the subscription keeps a local cache in sync, so a search is a scan
//! over memory, not a round-trip. Unlike the connections in
//! [`super::session`], this one is anonymous — it never calls `login`, so it
//! authenticates as nobody, which is fine because `player` is a public table.
//!
//! Reconnects lazily: if SpacetimeDB is unreachable at startup (or restarts
//! later), the first request that finds the socket down pays for a fresh
//! connection; until then handlers report `503` rather than blocking the
//! gateway's boot.

use tokio::sync::Mutex;

use super::connection::GatewayConnection;
use super::module_bindings::Player;

/// One `player` row as exposed by the public API. Everything here is already
/// public in the module — no credential, no email, nothing the authoritative
/// `account` table holds.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct PlayerEntry {
    /// Stable id of the character; this is the UUID the search endpoint
    /// returns and the detail endpoint accepts.
    pub character_id: uuid::Uuid,
    /// The account owning this character.
    pub account_id: u64,
    pub display_name: String,
    pub online: bool,
    /// RFC 3339, from the module's `Timestamp`. Empty only if the timestamp
    /// somehow cannot be formatted, which cannot happen for module-written
    /// values.
    pub last_seen: String,
}

impl From<Player> for PlayerEntry {
    fn from(row: Player) -> Self {
        Self {
            character_id: uuid::Uuid::from_u128(row.character_id.as_u128()),
            account_id: row.account_id,
            display_name: row.display_name,
            online: row.online,
            last_seen: row.last_seen.to_rfc3339().unwrap_or_default(),
        }
    }
}

/// Case-insensitive substring match on the module's pre-normalized name key.
/// `needle` must already be trimmed and lowercased by the caller.
fn matches(needle: &str, row: &Player) -> bool {
    row.normalized_name.contains(needle)
}

/// The gateway's shared reader of the public `player` table. Construct once
/// at boot and keep in the app state; it connects on first use and repairs
/// itself whenever SpacetimeDB goes away and comes back.
pub struct PlayerDirectory {
    uri: String,
    module: String,
    /// The one shared connection, behind a mutex so concurrent requests never
    /// race two reconnects: the first caller pays the handshake, the rest clone
    /// the result.
    slot: Mutex<Option<GatewayConnection>>,
}

impl PlayerDirectory {
    pub fn new(uri: impl Into<String>, module: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            module: module.into(),
            slot: Mutex::new(None),
        }
    }

    /// A live connection, connecting (or reconnecting) if the current one is
    /// missing or dead. Holding the lock across `connect` is intentional: it
    /// serializes reconnects instead of letting N concurrent requests open N
    /// connections.
    async fn live_connection(&self) -> Result<GatewayConnection, String> {
        let mut slot = self.slot.lock().await;
        match slot.as_ref() {
            Some(connection) if !connection.is_closed() => Ok(connection.clone()),
            _ => {
                let connection = GatewayConnection::connect(&self.uri, &self.module).await?;
                *slot = Some(connection.clone());
                Ok(connection)
            }
        }
    }

    /// Characters whose name contains `needle` (case-insensitive), ordered
    /// by display name, then paged: `offset` skips, `limit` caps. An empty
    /// `needle` matches every character — i.e. "list them all". Paging
    /// happens after the sort, so a page is stable while the cache only
    /// changes by rows being created or deleted.
    pub async fn search(
        &self,
        needle: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<PlayerEntry>, String> {
        let connection = self.live_connection().await?;
        let needle = needle.trim().to_lowercase();

        let mut entries: Vec<PlayerEntry> = connection
            .players()
            .into_iter()
            .filter(|row| matches(&needle, row))
            .map(PlayerEntry::from)
            .collect();
        entries.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        Ok(entries.into_iter().skip(offset).take(limit).collect())
    }

    /// One character by id, `None` if no such row exists.
    pub async fn get(&self, character_id: uuid::Uuid) -> Result<Option<PlayerEntry>, String> {
        let character_id = spacetimedb_sdk::Uuid::from_u128(character_id.as_u128());
        let connection = self.live_connection().await?;
        Ok(connection
            .players()
            .into_iter()
            .find(|row| row.character_id == character_id)
            .map(PlayerEntry::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(name: &str) -> Player {
        Player {
            character_id: spacetimedb_sdk::Uuid::from_u128(0x0189_0000_0000_4000_8000_0000_0000_0001),
            account_id: 7,
            normalized_name: name.to_lowercase(),
            display_name: name.to_string(),
            entity_id: 1,
            online: false,
            last_seen: spacetimedb_sdk::Timestamp::UNIX_EPOCH,
        }
    }

    #[test]
    fn empty_needle_matches_everything() {
        assert!(matches("", &player("aragorn")));
    }

    #[test]
    fn needle_matches_substring_of_normalized_name() {
        let row = player("Aragorn");
        assert!(matches("rag", &row));
        assert!(!matches("legolas", &row));
    }

    #[test]
    fn needle_is_case_insensitive_because_it_is_pre_lowercased() {
        // The caller lowercases; `matches` itself only ever sees an already
        // normalized needle, which is what makes the normalized_name column a
        // valid match key.
        let row = player("Aragorn");
        assert!(matches(&"aragorn".to_lowercase(), &row));
    }

    #[test]
    fn player_entry_formats_last_seen_as_rfc3339() {
        let mut row = player("Aragorn");
        row.last_seen = spacetimedb_sdk::Timestamp::from_micros_since_unix_epoch(0);
        let entry = PlayerEntry::from(row);
        assert!(entry.last_seen.starts_with("1970-01-01T"));
    }

    #[test]
    fn player_entry_carries_the_uuid_across_the_crate_boundary() {
        let entry = PlayerEntry::from(player("Aragorn"));
        assert_eq!(
            entry.character_id.to_string(),
            "01890000-0000-4000-8000-000000000001"
        );
    }
}
