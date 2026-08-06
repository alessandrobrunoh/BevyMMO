//! Migrazioni SeaORM versionate, applicate dal [`PersistencePlugin`] durante l'avvio.
//!
//! [`PersistencePlugin`]: crate::plugins::persistence::plugin::PersistencePlugin

use sea_orm_migration::prelude::*;

mod m20260805_000001_create_players;
mod m20260805_000002_create_player_stats;
mod m20260805_000003_create_player_spells;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260805_000001_create_players::Migration),
            Box::new(m20260805_000002_create_player_stats::Migration),
            Box::new(m20260805_000003_create_player_spells::Migration),
        ]
    }
}
