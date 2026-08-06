use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260806_000006_create_player_hotbar"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PlayerHotbar::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayerHotbar::PlayerId)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PlayerHotbar::QSpell).text())
                    .col(ColumnDef::new(PlayerHotbar::WSpell).text())
                    .col(ColumnDef::new(PlayerHotbar::ESpell).text())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-player_hotbar-player_id")
                            .from(PlayerHotbar::Table, PlayerHotbar::PlayerId)
                            .to(Players::Table, Players::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        let backfill_from_old = r#"
            INSERT INTO player_hotbar (player_id, q_spell, w_spell, e_spell)
            SELECT
                player_id,
                MAX(CASE WHEN slot_index = 0 THEN spell_id END) AS q_spell,
                MAX(CASE WHEN slot_index = 1 THEN spell_id END) AS w_spell,
                MAX(CASE WHEN slot_index = 2 THEN spell_id END) AS e_spell
            FROM player_spells
            GROUP BY player_id
            ON CONFLICT (player_id) DO NOTHING;
        "#;
        manager
            .get_connection()
            .execute_unprepared(backfill_from_old)
            .await?;

        let backfill_defaults = r#"
            INSERT INTO player_hotbar (player_id, q_spell, w_spell, e_spell)
            SELECT id, 'attack', 'fireball', 'healing_circle'
            FROM players
            WHERE NOT EXISTS (
                SELECT 1 FROM player_hotbar WHERE player_hotbar.player_id = players.id
            );
        "#;
        manager
            .get_connection()
            .execute_unprepared(backfill_defaults)
            .await?;

        manager
            .drop_table(Table::drop().table(PlayerSpells::Table).to_owned())
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
                    .unique()
                    .to_owned(),
            )
            .await?;

        let backfill_from_hotbar = r#"
            INSERT INTO player_spells (player_id, spell_id, slot_index)
            SELECT player_id, q_spell, 0 FROM player_hotbar WHERE q_spell IS NOT NULL
            UNION ALL
            SELECT player_id, w_spell, 1 FROM player_hotbar WHERE w_spell IS NOT NULL
            UNION ALL
            SELECT player_id, e_spell, 2 FROM player_hotbar WHERE e_spell IS NOT NULL;
        "#;
        manager
            .get_connection()
            .execute_unprepared(backfill_from_hotbar)
            .await?;

        manager
            .drop_table(Table::drop().table(PlayerHotbar::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum PlayerHotbar {
    Table,
    PlayerId,
    QSpell,
    WSpell,
    ESpell,
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
