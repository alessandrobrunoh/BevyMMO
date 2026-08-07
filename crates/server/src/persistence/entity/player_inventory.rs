use crate::persistence::entity::player;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// SeaORM model for `player_inventory`.
///
/// The whole 10-slot inventory is stored as a JSON array (`slots_json`) to
/// keep the fixed-size layout atomic: one row per player, no join table.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "player_inventory")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub player_id: Uuid,
    /// JSON array of 10 entries, each `null` or a serialized `ItemId`.
    pub slots_json: String,
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
