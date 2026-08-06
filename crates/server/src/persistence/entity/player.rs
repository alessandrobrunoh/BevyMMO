//! SeaORM entity for the `players` table.
//!
//! Inferred schema (PostgreSQL):
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
//! `normalized_name` is unique at DB level to ensure correctness of
//! `find_or_create` under concurrent inserts; the application-side normalization
//! helper is located in [`crate::plugins::persistence::normalize_name`].

use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// Domain alias for the read model.
///
/// `PlayerRecord` is what callers receive from repository reads;
/// mutations pass through [`PlayerActiveModel`].
pub type PlayerRecord = Model;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "players")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Stable lookup key (lowercase, trimmed). Supported by a UNIQUE index.
    pub normalized_name: String,
    /// Free-form display name, as provided by the player.
    pub display_name: String,
    /// Player world position, decomposed into axes (f32 corresponds to Bevy's `Vec3`).
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

