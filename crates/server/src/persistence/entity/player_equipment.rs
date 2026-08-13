use crate::persistence::entity::player;
use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// SeaORM model for `player_equipment`.
///
/// One row per player, one nullable column per [`bevymmo_shared::items::components::EquipSlot`]
/// variant. `NULL` means the slot is empty.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "player_equipment")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub player_id: Uuid,
    pub bag: Option<String>,
    pub helmet: Option<String>,
    pub cape: Option<String>,
    pub weapon: Option<String>,
    pub armor: Option<String>,
    pub offhand: Option<String>,
    pub potion: Option<String>,
    pub shoes: Option<String>,
    pub food: Option<String>,
    pub mount: Option<String>,
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
