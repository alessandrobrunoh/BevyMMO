//! Migrazioni SeaORM versionate, applicate dal [`PersistencePlugin`] durante l'avvio.
//!
//! [`PersistencePlugin`]: crate::plugins::persistence::plugin::PersistencePlugin

use sea_orm_migration::prelude::*;

mod m20260805_000001_create_players;
mod m20260805_000002_create_player_stats;
mod m20260805_000003_create_player_spells;
mod m20260805_000004_rename_fireball_spell_to_ray_of_light;
mod m20260806_000005_rename_followball_spell_to_fireball;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260805_000001_create_players::Migration),
            Box::new(m20260805_000002_create_player_stats::Migration),
            Box::new(m20260805_000003_create_player_spells::Migration),
            Box::new(m20260805_000004_rename_fireball_spell_to_ray_of_light::Migration),
            Box::new(m20260806_000005_rename_followball_spell_to_fireball::Migration),
        ]
    }
}
