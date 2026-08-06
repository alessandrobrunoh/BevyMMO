//! Migrazioni SeaORM versionate, applicate dal server durante l'avvio.

use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260805_000001_create_players::Migration),
            Box::new(m20260805_000002_create_player_stats::Migration),
        ]
    }
}

mod m20260805_000001_create_players {
    use super::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

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
}

mod m20260805_000002_create_player_stats {
    use super::*;

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(PlayerStats::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(PlayerStats::PlayerId)
                                .uuid()
                                .not_null()
                                .primary_key(),
                        )
                        .col(
                            ColumnDef::new(PlayerStats::CurrentHealth)
                                .float()
                                .not_null(),
                        )
                        .col(ColumnDef::new(PlayerStats::MaxHealth).float().not_null())
                        .col(ColumnDef::new(PlayerStats::MaxMana).float().not_null())
                        .col(
                            ColumnDef::new(PlayerStats::ManaRegeneration)
                                .float()
                                .not_null(),
                        )
                        .col(ColumnDef::new(PlayerStats::Armor).float().not_null())
                        .col(
                            ColumnDef::new(PlayerStats::MovementSpeed)
                                .float()
                                .not_null(),
                        )
                        .col(ColumnDef::new(PlayerStats::AttackPower).float().not_null())
                        .foreign_key(
                            ForeignKey::create()
                                .name("fk-player_stats-player_id")
                                .from(PlayerStats::Table, PlayerStats::PlayerId)
                                .to(Players::Table, Players::Id)
                                .on_delete(ForeignKeyAction::Cascade),
                        )
                        .to_owned(),
                )
                .await?;

            // Backfill: inserisce stats di default per ogni player esistente
            let backfill = r#"
                INSERT INTO player_stats (player_id, current_health, max_health, max_mana, mana_regeneration, armor, movement_speed, attack_power)
                SELECT id, 100.0, 100.0, 100.0, 5.0, 25.0, 0.15, 10.0
                FROM players
                ON CONFLICT (player_id) DO NOTHING
            "#;
            manager
                .get_connection()
                .execute_unprepared(backfill)
                .await?;

            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(PlayerStats::Table).to_owned())
                .await
        }
    }

    #[derive(DeriveIden)]
    enum PlayerStats {
        Table,
        PlayerId,
        CurrentHealth,
        MaxHealth,
        MaxMana,
        ManaRegeneration,
        Armor,
        MovementSpeed,
        AttackPower,
    }

    // Riutilizza l'enum Players per la foreign key
    #[derive(DeriveIden)]
    enum Players {
        Table,
        Id,
    }
}
