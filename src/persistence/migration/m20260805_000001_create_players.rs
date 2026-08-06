use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260805_000001_create_players"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Players::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Players::Id).uuid().not_null().primary_key())
                    .col(
                        ColumnDef::new(Players::NormalizedName)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Players::DisplayName).text().not_null())
                    .col(ColumnDef::new(Players::PosX).float().not_null())
                    .col(ColumnDef::new(Players::PosY).float().not_null())
                    .col(ColumnDef::new(Players::PosZ).float().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Players::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Players {
    Table,
    Id,
    NormalizedName,
    DisplayName,
    PosX,
    PosY,
    PosZ,
}
