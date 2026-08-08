//! PostgreSQL persistence layer (SeaORM).
//!
//! This module is intentionally decoupled from the Bevy game loop and network
//! layer. All repository methods are `async` and must be driven by a non-blocking
//! runtime (e.g., a Tokio task spawned outside the main schedule).
//! Do not invoke `.await` on repository methods from within Bevy systems running on
//! render/fixed-update schedules.
//!
//! The module is **not** initialized here. Constructing the
//! [`DatabaseConnection`] and attaching it to Bevy resources is the responsibility
//! of [`crate::persistence::plugin::PersistencePlugin`].
//! SeaORM migrations are defined in [`crate::migrations`], separate from plugin logic.

pub mod entity;
pub mod error;
pub mod plugin;
pub mod repository;

pub use entity::player::PlayerRecord;
pub use entity::player_stats::Model as PlayerStatsRecord;
pub use error::PersistenceError;
pub use plugin::{PersistencePlugin, PersistenceRuntime, PlayerStore, PropOverrideStore};
pub use repository::player::PersistedPlayerSnapshot;
pub use repository::prop_override::PropOverrideRepository;

/// Normalizes a player name for use as a unique lookup key.
///
/// The current normalization performs lowercasing + trimming of whitespace, kept
/// pure and deterministic so that the same input always produces the same key.
/// Centralizing it here ensures that callers of [`PlayerRepository::find_or_create`]
/// cannot accidentally diverge on key representation.
pub fn normalize_name(name: &str) -> String {
    name.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::normalize_name;

    #[test]
    fn normalize_lowercases_and_trims() {
        assert_eq!(normalize_name("  Alice "), "alice");
        assert_eq!(normalize_name("BOB"), "bob");
        assert_eq!(normalize_name("\tCarol\n"), "carol");
    }

    #[test]
    fn normalize_is_idempotent() {
        let once = normalize_name("  Dave ");
        let twice = normalize_name(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn normalize_collapses_only_whitespace_and_case() {
        // Internal whitespace is preserved (only outer trim is applied),
        // so "A B" and "a b" share key but "a  b" does not.
        assert_eq!(normalize_name("A B"), "a b");
        assert_ne!(normalize_name("A B"), normalize_name("A  B"));
    }
}
