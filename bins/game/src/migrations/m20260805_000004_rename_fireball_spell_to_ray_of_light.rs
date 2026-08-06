//! Migration: renames spell id `fireball` to `ray_of_light`.
//!
//! Following the spell refactor (from projectile to beam), the textual id
//! used in `Spellbook` and in the `player_spells` table has changed. Without
//! this migration, existing players would end up with an orphan slot
//! (id no longer registered in `SpellRegistry`) and without the new ray.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260805_000004_rename_fireball_spell_to_ray_of_light"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // ON CONFLICT DO NOTHING to handle DBs where `ray_of_light` already
        // exists (e.g. manual seed): we don't block the migration.
        let rename = r#"
            UPDATE player_spells
            SET spell_id = 'ray_of_light'
            WHERE spell_id = 'fireball'
            AND NOT EXISTS (
                SELECT 1 FROM player_spells ps
                WHERE ps.player_id = player_spells.player_id
                  AND ps.spell_id = 'ray_of_light'
            )
        "#;
        manager.get_connection().execute_unprepared(rename).await?;

        // Cleanup residual duplicate slots in the edge case above.
        let cleanup = r#"
            DELETE FROM player_spells
            WHERE spell_id = 'fireball'
        "#;
        manager.get_connection().execute_unprepared(cleanup).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let revert = r#"
            UPDATE player_spells
            SET spell_id = 'fireball'
            WHERE spell_id = 'ray_of_light'
        "#;
        manager.get_connection().execute_unprepared(revert).await?;
        Ok(())
    }
}
