use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260805_000003_create_player_spells"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PlayerSpells::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PlayerSpells::PlayerId).uuid().not_null())
                    .col(ColumnDef::new(PlayerSpells::SpellId).text().not_null())
                    .col(ColumnDef::new(PlayerSpells::SlotIndex).integer().not_null())
                    .primary_key(
                        Index::create()
                            .name("pk-player_spells")
                            .col(PlayerSpells::PlayerId)
                            .col(PlayerSpells::SpellId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-player_spells-player_id")
                            .from(PlayerSpells::Table, PlayerSpells::PlayerId)
                            .to(Players::Table, Players::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-player_spells-player_slot")
                    .table(PlayerSpells::Table)
                    .col(PlayerSpells::PlayerId)
                    .col(PlayerSpells::SlotIndex)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        let backfill = r#"
            INSERT INTO player_spells (player_id, spell_id, slot_index)
            SELECT players.id, default_spells.spell_id, default_spells.slot_index
            FROM players
            CROSS JOIN (
                VALUES
                    ('attack', 0),
                    ('fireball', 1),
                    ('followball', 2),
                    ('healing_circle', 3),
                    ('meteorite', 4),
                    ('swift', 5)
            ) AS default_spells(spell_id, slot_index)
            ON CONFLICT (player_id, spell_id) DO NOTHING
        "#;
        manager
            .get_connection()
            .execute_unprepared(backfill)
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PlayerSpells::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum PlayerSpells {
    Table,
    PlayerId,
    SpellId,
    SlotIndex,
}

#[derive(DeriveIden)]
enum Players {
    Table,
    Id,
}
