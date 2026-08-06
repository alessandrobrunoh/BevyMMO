//! Migrazione: rinomina l'id della spell `fireball` in `ray_of_light`.
//!
//! A seguito del refactor della spell (da projectile a beam), l'id testuale
//! usato in `Spellbook` e nella tabella `player_spells` è cambiato. Senza
//! questa migrazione i player esistenti si ritroverebbero con uno slot
//! orfano (id non più registrato nel `SpellRegistry`) e senza il nuovo ray.

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
        // ON CONFLICT DO NOTHING per gestire DB in cui `ray_of_light` esistesse
        // già (es. seed manuale): non blocchiamo la migrazione.
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

        // Cleanup degli slot duplicati residuali nel caso limite sopra.
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
