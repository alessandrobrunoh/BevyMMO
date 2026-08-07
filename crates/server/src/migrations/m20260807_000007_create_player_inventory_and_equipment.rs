use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260807_000007_create_player_inventory_and_equipment"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Inventory is stored as one JSON array of 10 entries (null | item id)
        // per player. The fixed-size layout matches `Inventory::slots`, so a
        // single atomic row avoids a 10-row join table for no query benefit.
        manager
            .create_table(
                Table::create()
                    .table(PlayerInventory::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayerInventory::PlayerId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PlayerInventory::SlotsJson).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-player_inventory-player_id")
                            .from(PlayerInventory::Table, PlayerInventory::PlayerId)
                            .to(Players::Table, Players::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PlayerEquipment::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayerEquipment::PlayerId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PlayerEquipment::Weapon).text())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-player_equipment-player_id")
                            .from(PlayerEquipment::Table, PlayerEquipment::PlayerId)
                            .to(Players::Table, Players::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PlayerEquipment::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PlayerInventory::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum PlayerInventory {
    Table,
    PlayerId,
    SlotsJson,
}

#[derive(DeriveIden)]
enum PlayerEquipment {
    Table,
    PlayerId,
    Weapon,
}

#[derive(DeriveIden)]
enum Players {
    Table,
    Id,
}
