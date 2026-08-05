//! Tipi di errore per il layer di persistenza.
//!
//! Implementati a mano (senza `thiserror`) in modo che questa crate compili
//! senza aggiungere ulteriori dipendenze proc-macro oltre a quelle già richieste
//! da SeaORM. [`PersistenceError`] è `Send` + `Sync` così può attraversare i
//! confini di `tokio::spawn` e i bridge async di Bevy.

use std::fmt;

/// Errori restituiti dalle operazioni di persistenza.
#[derive(Debug)]
pub enum PersistenceError {
    /// Un'operazione database SeaORM è fallita (connessione, query, vincolo, ecc.).
    Db(sea_orm::DbErr),
    /// Una riga referenziata per id / chiave non è stata trovata in
    /// un'operazione che ne richiede l'esistenza (es. `save_position` su un id
    /// player sconosciuto).
    NotFound(String),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersistenceError::Db(err) => write!(f, "database error: {err}"),
            PersistenceError::NotFound(what) => write!(f, "not found: {what}"),
        }
    }
}

impl std::error::Error for PersistenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PersistenceError::Db(err) => Some(err),
            PersistenceError::NotFound(_) => None,
        }
    }
}

impl From<sea_orm::DbErr> for PersistenceError {
    fn from(err: sea_orm::DbErr) -> Self {
        PersistenceError::Db(err)
    }
}

/// Alias di convenienza usato nelle firme dei repository.
pub type PersistenceResult<T> = Result<T, PersistenceError>;
