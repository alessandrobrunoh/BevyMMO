//! Repository async supportati da SeaORM.
//!
//! Ogni repository possiede una (clonabile a basso costo, internamente con
//! `Arc`) [`sea_orm::DatabaseConnection`] ed espone metodi interamente
//! `async`. Nessuno di questi tipi dovrebbe essere costruito o atteso sul
//! main thread di Bevy.

pub mod player;
pub mod prop_override;
