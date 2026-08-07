use crate::persistence::entity::player;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// SeaORM model for `player_equipment`.
///
/// One row per player. `weapon` holds the equipped item id (or `NULL`).
/// Future equipment slots (helmet, chest, ...) get their own nullable columns
/// via future migrations; the table shape stays stable.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "player_equipment")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub player_id: Uuid,
    pub weapon: Option<String>,
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
