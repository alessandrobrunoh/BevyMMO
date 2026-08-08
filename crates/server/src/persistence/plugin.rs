//! Risorse Bevy che possiedono la connessione al database e il runtime async.

use bevy::prelude::*;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use tokio::runtime::Runtime;

use super::repository::player::PlayerRepository;
use super::repository::prop_override::PropOverrideRepository;
use crate::migrations::Migrator;

/// Accesso condiviso e asincrono ai player persistiti.
#[derive(Resource, Clone)]
pub struct PlayerStore(pub PlayerRepository);

/// Accesso condiviso e asincrono agli override dei prop persistiti.
#[derive(Resource, Clone)]
pub struct PropOverrideStore(pub PropOverrideRepository);

/// Runtime Tokio riservato al lavoro sul database fuori dalle schedule di Bevy.
#[derive(Resource)]
pub struct PersistenceRuntime(pub Runtime);

pub struct PersistencePlugin {
    database_url: String,
}

impl PersistencePlugin {
    pub fn new(database_url: String) -> Self {
        Self { database_url }
    }
}

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        let database_url = self.database_url.clone();
        let runtime = Runtime::new().expect("failed to create persistence Tokio runtime");
        let database = runtime.block_on(connect_and_migrate(&database_url));

        app.insert_resource(PlayerStore(PlayerRepository::new(database.clone())));
        app.insert_resource(PropOverrideStore(PropOverrideRepository::new(database)));
        app.insert_resource(PersistenceRuntime(runtime));
    }
}

async fn connect_and_migrate(database_url: &str) -> DatabaseConnection {
    let database = Database::connect(database_url)
        .await
        .expect("failed to connect to PostgreSQL using DATABASE_URL");

    Migrator::up(&database, None)
        .await
        .expect("failed to apply PostgreSQL migrations");

    ensure_player_stats_table(&database)
        .await
        .expect("failed to ensure PostgreSQL player_stats table");

    database
}

async fn ensure_player_stats_table(database: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    database
        .execute_unprepared(
            r#"
            CREATE TABLE IF NOT EXISTS player_stats (
                player_id UUID PRIMARY KEY REFERENCES players(id) ON DELETE CASCADE,
                current_health REAL NOT NULL,
                max_health REAL NOT NULL,
                max_mana REAL NOT NULL,
                mana_regeneration REAL NOT NULL,
                armor REAL NOT NULL,
                movement_speed REAL NOT NULL,
                attack_power REAL NOT NULL
            )
            "#,
        )
        .await?;

    database
        .execute_unprepared(
            r#"
            INSERT INTO player_stats (
                player_id,
                current_health,
                max_health,
                max_mana,
                mana_regeneration,
                armor,
                movement_speed,
                attack_power
            )
            SELECT id, 100.0, 100.0, 100.0, 5.0, 25.0, 0.15, 10.0
            FROM players
            ON CONFLICT (player_id) DO NOTHING
            "#,
        )
        .await?;

    Ok(())
}
