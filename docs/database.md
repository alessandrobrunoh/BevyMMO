# Database

Local PostgreSQL for multiplayer testing, exposed via Docker. Access is done through SeaORM.

## Prerequisites

- Docker with Compose v2 plugin (`docker compose ...`).

## Starting

```sh
cp .env.example .env
docker compose up -d
```

The `bevytest2_postgres` container exposes PostgreSQL on port `POSTGRES_PORT` (default `5432`). State is persisted in the `postgres_data` volume.

Verify that it is ready:

```sh
docker compose ps
```

The Compose healthcheck marks the service `healthy` when PostgreSQL accepts connections.

## Connection string

The app reads `DATABASE_URL` (see `.env`). For local defaults:

```
DATABASE_URL=postgresql://bevytest2:bevytest2_dev@localhost:5432/bevytest2
```

## Migrations

The server automatically applies pending SeaORM migrations on startup, before accepting connections. The first creates `players`, with unique `normalized_name` and persisted position.

Start PostgreSQL before the server:

```sh
docker compose up -d
cargo run -- server
```

History is recorded in the `seaql_migrations` table. Do not edit an already deployed migration: for future changes add a new migration in `src/persistence/migration.rs`.

## Stop / reset

```sh
# stops container keeping data
docker compose down

# stops and removes volume (full DB reset)
docker compose down -v
```
