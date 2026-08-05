# Database

PostgreSQL locale per il test multiplayer, esposto via Docker. L'accesso avviene tramite SeaORM.

## Prerequisiti

- Docker con il plugin Compose v2 (`docker compose ...`).

## Avvio

```sh
cp .env.example .env
docker compose up -d
```

Il container `bevytest2_postgres` espone PostgreSQL sulla porta `POSTGRES_PORT` (default `5432`). Lo stato è persistito nel volume `postgres_data`.

Verifica che sia pronto:

```sh
docker compose ps
```

La healthcheck di Compose marca il service `healthy` quando PostgreSQL accetta connessioni.

## Connection string

L'app legge `DATABASE_URL` (vedi `.env`). Per i default locali:

```
DATABASE_URL=postgresql://bevytest2:bevytest2_dev@localhost:5432/bevytest2
```

## Migrazioni

Il server applica automaticamente le migrazioni SeaORM pendenti all'avvio, prima di accettare connessioni. La prima crea `players`, con `normalized_name` univoco e posizione persistita.

Avvia PostgreSQL prima del server:

```sh
docker compose up -d
cargo run -- server
```

Lo storico è registrato nella tabella `seaql_migrations`. Non modificare una migration già distribuita: per cambiamenti futuri aggiungi una nuova migration in `src/persistence/migration.rs`.

## Stop / reset

```sh
# ferma il container mantenendo i dati
docker compose down

# ferma e cancella il volume (reset completo del DB)
docker compose down -v
```
