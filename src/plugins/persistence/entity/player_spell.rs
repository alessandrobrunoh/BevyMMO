//! SeaORM entity for the `player_spells` table.
//!
//! The table keeps the authoritative spellbook outside replicated ECS state, so
//! reconnecting players regain the same spell unlocks and hotbar order.

use crate::plugins::persistence::entity::player;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "player_spells")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub player_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub spell_id: String,
    pub slot_index: i32,
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
