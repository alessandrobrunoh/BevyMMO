//! Risorse Bevy che possiedono la connessione al database e il runtime async.

use std::env;

use bevy::prelude::*;
use sea_orm::{Database, DatabaseConnection};
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

    database
}
