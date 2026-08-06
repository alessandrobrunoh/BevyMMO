//! Migration: renames spell id `followball` to `fireball`.
//!
//! The homing spell has been renamed to `Fireball` (and made much faster).
//! Without this migration, existing players would keep a slot with id
//! `followball`, orphaned relative to `SpellRegistry` which now registers `fireball`.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260806_000005_rename_followball_spell_to_fireball"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // If `fireball` is already present for the player (manual seed),
        // we avoid primary key violation (player_id, spell_id).
        let rename = r#"
            UPDATE player_spells
            SET spell_id = 'fireball'
            WHERE spell_id = 'followball'
            AND NOT EXISTS (
                SELECT 1 FROM player_spells ps
                WHERE ps.player_id = player_spells.player_id
                  AND ps.spell_id = 'fireball'
            )
        "#;
        manager.get_connection().execute_unprepared(rename).await?;

        // Cleanup residual slots in the edge case above.
        let cleanup = r#"
            DELETE FROM player_spells
            WHERE spell_id = 'followball'
        "#;
        manager.get_connection().execute_unprepared(cleanup).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let revert = r#"
            UPDATE player_spells
            SET spell_id = 'followball'
            WHERE spell_id = 'fireball'
        "#;
        manager.get_connection().execute_unprepared(revert).await?;
        Ok(())
    }
}
