//! Layer di persistenza PostgreSQL (SeaORM).
//!
//! Questo modulo è volutamente disaccoppiato dal game loop Bevy e dallo strato
//! di rete. Tutti i metodi del repository sono `async` e devono essere guidati
//! da un runtime non bloccante (es. un task Tokio avviato fuori dalla schedule
//! principale). Non invocare `.await` sui metodi del repository da dentro
//! sistemi Bevy in esecuzione sulla schedule di render/fixed-update.
//!
//! Il modulo **non** è inizializzato qui. La costruzione della
//! [`DatabaseConnection`] e il collegamento alle risorse Bevy sono
//! responsabilità del [`crate::plugins::persistence::plugin::PersistencePlugin`].
//! Le migrazioni SeaORM sono definite in [`crate::migrations`], separate dalla
//! logica di plugin.

pub mod entity;
pub mod error;
pub mod plugin;
pub mod repository;

pub use entity::player::PlayerRecord;
pub use entity::player_stats::Model as PlayerStatsRecord;
pub use error::PersistenceError;
pub use plugin::{PersistencePlugin, PersistenceRuntime, PlayerStore};
pub use repository::player::PersistedPlayerSnapshot;

/// Normalizza il nome di un giocatore per usarlo come chiave di lookup univoca.
///
/// La normalizzazione attuale è un lowercase + trim degli spazi, mantenuta
/// pura e deterministica così che lo stesso input produca sempre la stessa
/// chiave. Centralizzarla qui garantisce che i chiamanti di
/// [`PlayerRepository::find_or_create`] non possano divergere accidentalmente
/// sulla forma della chiave.
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
        // Gli spazi bianchi interni sono preservati (viene applicato solo il
        // trim esterno), quindi "A B" e "a b" condividono la chiave ma "a  b" no.
        assert_eq!(normalize_name("A B"), "a b");
        assert_ne!(normalize_name("A B"), normalize_name("A  B"));
    }
}
