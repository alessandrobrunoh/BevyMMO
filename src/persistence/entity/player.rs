//! Entità SeaORM per la tabella `players`.
//!
//! Schema inferito (PostgreSQL):
//!
//! ```sql
//! CREATE TABLE players (
//!     id               UUID PRIMARY KEY,
//!     normalized_name  TEXT NOT NULL UNIQUE,
//!     display_name     TEXT NOT NULL,
//!     pos_x            REAL NOT NULL,
//!     pos_y            REAL NOT NULL,
//!     pos_z            REAL NOT NULL
//! );
//! ```
//!
//! `normalized_name` è unica a livello DB per garantire la correttezza di
//! `find_or_create` sotto inserimenti concorrenti; l'helper di normalizzazione
//! lato applicazione si trova in [`crate::persistence::normalize_name`].

use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// Alias di dominio per il model di lettura.
///
/// `PlayerRecord` è ciò che i chiamanti ricevono dalle letture del repository;
/// le mutazioni passano tramite [`PlayerActiveModel`].
pub type PlayerRecord = Model;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "players")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Chiave di lookup stabile (lowercase, trimmata). Supportata da un indice UNIQUE.
    pub normalized_name: String,
    /// Display name in formato libero, come fornito dal player.
    pub display_name: String,
    /// Posizione mondiale del player, decomposta per assi (f32 corrisponde a `Vec3` di Bevy).
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
