use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260813_000009_add_equipment_slots"
    }
}

/// Grows `player_equipment` from a single `weapon` column to one nullable
/// column per [`bevymmo_shared::items::components::EquipSlot`] variant.
///
/// Existing rows keep their `weapon` value; every new column defaults to
/// `NULL` (empty slot), matching `Equipment::default()`.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for column in [
            PlayerEquipment::Bag,
            PlayerEquipment::Helmet,
            PlayerEquipment::Cape,
            PlayerEquipment::Armor,
            PlayerEquipment::Offhand,
            PlayerEquipment::Potion,
            PlayerEquipment::Shoes,
            PlayerEquipment::Food,
            PlayerEquipment::Mount,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(PlayerEquipment::Table)
                        .add_column(ColumnDef::new(column).text())
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for column in [
            PlayerEquipment::Bag,
            PlayerEquipment::Helmet,
            PlayerEquipment::Cape,
            PlayerEquipment::Armor,
            PlayerEquipment::Offhand,
            PlayerEquipment::Potion,
            PlayerEquipment::Shoes,
            PlayerEquipment::Food,
            PlayerEquipment::Mount,
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(PlayerEquipment::Table)
                        .drop_column(column)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}

#[derive(DeriveIden)]
enum PlayerEquipment {
    Table,
    Bag,
    Helmet,
    Cape,
    Armor,
    Offhand,
    Potion,
    Shoes,
    Food,
    Mount,
}
