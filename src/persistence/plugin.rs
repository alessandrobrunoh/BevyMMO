//! Risorse Bevy che possiedono la connessione al database e il runtime async.

use std::env;

use bevy::prelude::*;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use tokio::runtime::Runtime;

use super::migration::Migrator;
use super::repository::player::PlayerRepository;

/// Accesso condiviso e asincrono ai player persistiti.
#[derive(Resource, Clone)]
pub struct PlayerStore(pub PlayerRepository);

/// Runtime Tokio riservato al lavoro sul database fuori dalle schedule di Bevy.
#[derive(Resource)]
pub struct PersistenceRuntime(pub Runtime);

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL is required when starting a server; copy .env.example to .env");
        let runtime = Runtime::new().expect("failed to create persistence Tokio runtime");
        let database = runtime.block_on(connect_and_migrate(&database_url));

        app.insert_resource(PlayerStore(PlayerRepository::new(database)));
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
