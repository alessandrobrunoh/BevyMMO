//! Error types for the persistence layer.
//!
//! Implemented manually (without `thiserror`) so that this crate compiles
//! without adding further proc-macro dependencies beyond those already required
//! by SeaORM. [`PersistenceError`] is `Send` + `Sync` so it can cross
//! `tokio::spawn` boundaries and Bevy async bridges.

use std::fmt;

/// Errors returned by persistence operations.
#[derive(Debug)]
pub enum PersistenceError {
    /// A SeaORM database operation failed (connection, query, constraint, etc.).
    Db(sea_orm::DbErr),
    /// A row referenced by id / key was not found in
    /// an operation that requires its existence (e.g. `save_position` on an
    /// unknown player id).
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

/// Convenience alias used in repository signatures.
pub type PersistenceResult<T> = Result<T, PersistenceError>;

