//! SeaORM entity for the `player_stats` table.
//!
//! Inferred schema (PostgreSQL):
//!
//! ```sql
//! CREATE TABLE player_stats (
//!     player_id            UUID PRIMARY KEY REFERENCES players(id) ON DELETE CASCADE,
//!     current_health       REAL NOT NULL,
//!     max_health           REAL NOT NULL,
//!     max_mana             REAL NOT NULL,
//!     mana_regeneration    REAL NOT NULL,
//!     armor                REAL NOT NULL,
//!     movement_speed       REAL NOT NULL,
//!     attack_power        REAL NOT NULL
//! );
//! ```
//!
//! `player_id` is the primary key and foreign key to the `players` table.
//! The `BelongsTo` relationship allows navigating from the stats record to the player.


use crate::plugins::persistence::entity::player;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "player_stats")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub player_id: Uuid,
    pub current_health: f32,
    pub max_health: f32,
    pub max_mana: f32,
    pub mana_regeneration: f32,
    pub armor: f32,
    pub movement_speed: f32,
    pub attack_power: f32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "player::Entity",
        from = "Column::PlayerId",
        to = "player::Column::Id"
    )]
    Player,
}

impl Related<player::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Player.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
