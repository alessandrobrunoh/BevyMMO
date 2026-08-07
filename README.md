# BevyMMO

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024-orange.svg)](https://www.rust-lang.org/)
[![Bevy](https://img.shields.io/badge/Bevy-0.19-blueviolet.svg)](https://bevyengine.org/)

BevyMMO is an open-source multiplayer MMO prototype built with [Bevy](https://bevyengine.org/) and [Lightyear](https://github.com/cBournhonesque/lightyear). The project uses Bevy ECS for gameplay composition, Lightyear for networking, server-authoritative simulation, client-side presentation, and PostgreSQL persistence through SeaORM.

The default `game` binary is a combined build that can run as a client, a dedicated server, or a host-client process depending on the CLI mode selected at startup.

## Table of Contents

- [Features](#features)
- [Technology Stack](#technology-stack)
- [Repository Layout](#repository-layout)
- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Run Modes](#run-modes)
- [Configuration](#configuration)
- [Database](#database)
- [Docker](#docker)
- [Development Workflow](#development-workflow)
- [Architecture Overview](#architecture-overview)
- [Controls](#controls)
- [Adding Gameplay Content](#adding-gameplay-content)
- [Troubleshooting](#troubleshooting)
- [Further Documentation](#further-documentation)
- [Contributing](#contributing)
- [Security and Secrets](#security-and-secrets)
- [License](#license)

## Features

- Multiplayer networking with Lightyear.
- Dedicated server, standalone client, and embedded host-client modes.
- Server-authoritative gameplay systems gated by explicit application roles.
- Client-side rendering and UI built from replicated gameplay state.
- Replicated gameplay entities with shared health, stats, position, color, and lifecycle state.
- Player movement with networked input and prediction/interpolation support.
- Targeting and spell casting systems.
- PostgreSQL-backed persistence using SeaORM.
- Automatic SeaORM migration execution at server startup.
- Layered runtime configuration for development, production, local overrides, and environment variables.
- Docker Compose setup for local PostgreSQL and production-style server deployment.

## Technology Stack

| Area | Technology |
| --- | --- |
| Language | Rust 2021 |
| Game engine | Bevy `0.19` |
| Networking | Lightyear `0.28` |
| CLI | Clap |
| Configuration | `config` crate with TOML and environment sources |
| Database | PostgreSQL |
| ORM / migrations | SeaORM and SeaORM Migration |
| Async runtime | Tokio |
| Containers | Docker and Docker Compose |

## Repository Layout

```text
.
├── bins/
│   └── game/               # Composition root CLI binary
├── config/                 # Runtime configuration files
│   ├── default.toml         # Shared defaults
│   ├── development.toml     # Development environment overrides
│   ├── production.toml      # Production environment overrides
│   └── local.toml.example   # Template for gitignored local overrides
├── crates/
│   ├── client/             # Client-only input, targeting, movement logic
│   ├── editor/             # Editor-mode plugin placeholder
│   ├── presentation/       # Visual presentation, rendering, UI widgets, scenes
│   ├── server/             # Authoritative simulation, persistence, migrations
│   └── shared/             # Shared entities, protocol, settings, state
├── docs/                   # Architecture and contributor documentation
├── plans/                  # Design notes and feature/refactor plans
├── Cargo.toml              # Workspace root Cargo manifest
├── docker-compose.yml
├── Dockerfile
└── LICENSE
```

## Prerequisites

Install the following tools before running the project locally:

- Rust stable toolchain with Cargo.
- Docker with Docker Compose v2 for the local PostgreSQL service.
- A GPU/graphics environment supported by Bevy when running client modes.
- Network access to UDP port `5051` when connecting clients to a remote server.

Recommended checks:

```sh
rustc --version
cargo --version
docker --version
docker compose version
```

## Quick Start

### 1. Clone and enter the repository

```sh
git clone https://github.com/alessandrobrunoh/BevyMMO.git
cd BevyMMO
```

If you are using a fork, replace the URL with your fork URL.

### 2. Prepare local environment files

```sh
cp .env.example .env
cp config/local.toml.example config/local.toml
```

`config/local.toml` is intended for machine-specific overrides and secrets. Do not commit real credentials.

### 3. Start PostgreSQL

```sh
docker compose up -d postgres
```

Check that the database container is healthy:

```sh
docker compose ps
```

### 4. Run the server

```sh
cargo run -- server
```

The server applies pending database migrations automatically during startup.

### 5. Run a client in another terminal

```sh
cargo run -- client
```

For local single-process testing, you can also run both server and client in one process:

```sh
cargo run -- host-client
```

## Run Modes

The `game` binary requires one of three subcommands.

### Client

Runs only client networking, UI, scenes, and rendering.

```sh
cargo run -- client
```

Optional arguments:

```sh
cargo run -- client --client-id 42 --server-addr 127.0.0.1:5051
```

Short form for `client_id` is also available:

```sh
cargo run -- client -c 42
```

### Server

Runs a headless authoritative server. This mode requires `DATABASE_URL` to be available from configuration or environment variables.

```sh
cargo run -- server
```

Optional bind address override:

```sh
cargo run -- server --bind-addr 0.0.0.0:5051
```

### Host Client

Runs server and client roles in the same process. This is useful for local gameplay testing.

```sh
cargo run -- host-client
```

Optional arguments:

```sh
cargo run -- host-client --client-id 42 --server-addr 127.0.0.1:5051
```

## Cargo Features

The default feature set builds a “fat” binary containing client and server logic:

```toml
default = [
  "client",
  "server",
  "netcode",
  "udp",
  "interpolation",
  "prediction",
  "replication",
  "input_native"
]
```

For a dedicated production server without client rendering/UI dependencies, build with server-only features:

```sh
cargo build --release --no-default-features --features server,netcode,udp,replication --bin game
```

## Configuration

Configuration is loaded in layers. Later sources override earlier ones:

```text
config/default.toml
  < config/<APP_ENV>.toml
  < config/local.toml
  < environment variables
```

`APP_ENV` defaults to `development`.

Examples:

```sh
APP_ENV=production cargo run -- server
DATABASE_URL=postgresql://user:password@localhost:5433/bevy_mmo cargo run -- server
```

Important settings:

| Setting | Description | Default / Development value |
| --- | --- | --- |
| `tick_rate` | Fixed simulation tick rate in Hz. | `60.0` |
| `log_filter` | Bevy/Rust log filter syntax. | `warn,bevy_lightyear_game=debug,lightyear=info` |
| `server.bind_addr` | UDP address the server binds to. | `0.0.0.0:5051` |
| `client.server_addr` | UDP server address the client connects to. | `127.0.0.1:5051` in development |
| `client.client_addr` | Local client UDP bind address. | `0.0.0.0:0` |
| `database_url` | PostgreSQL connection string. Required for server modes. | Provided by development/local/env config |

Use `config/local.toml` for local overrides, for example:

```toml
database_url = "postgresql://bevy:bevy@127.0.0.1:5433/bevy_mmo"

[client]
server_addr = "127.0.0.1:5051"
```

## Database

The server uses PostgreSQL through SeaORM.

### Local database startup

```sh
cp .env.example .env
docker compose up -d postgres
```

### Migrations

Migrations live in `crates/server/src/migrations/` and are applied automatically when the server starts. There is no manual SeaORM CLI step required for existing migrations.

Current migration responsibilities include:

- Creating persisted player records.
- Creating persisted player stats.
- Tracking applied migrations in SeaORM’s migration history table.

Start PostgreSQL before launching server modes:

```sh
docker compose up -d postgres
cargo run -- server
```

### Resetting local data

Stop containers while keeping the database volume:

```sh
docker compose down
```

Stop containers and delete the PostgreSQL volume:

```sh
docker compose down -v
```

## Docker

`docker-compose.yml` defines:

- `postgres`: PostgreSQL 16 with a health check and persistent volume.
- `server`: containerized dedicated server image `ghcr.io/alessandrobrunoh/bevy_mmo:latest`.

Start only PostgreSQL for local development:

```sh
docker compose up -d postgres
```

Start the full Compose stack:

```sh
docker compose up -d
```

The dedicated server exposes UDP port `5051`.

## Development Workflow

Run tests:

```sh
cargo test
```

Run Clippy with warnings as errors:

```sh
cargo clippy -- -D warnings
```

Format code:

```sh
cargo fmt
```

Common local loop:

```sh
docker compose up -d postgres
cargo test
cargo clippy -- -D warnings
cargo run -- host-client
```

## Architecture Overview

The application is built from Bevy plugins. Each plugin owns one observable capability such as networking, movement, entities, persistence, spells, rendering, scenes, or UI.

```mermaid
flowchart TD
    CLI[CLI mode] --> Bootstrap[bins/game main.rs bootstrap]
    Bootstrap --> AppMode[AppMode]
    AppMode --> Network[Network plugins]
    AppMode --> Gameplay[Gameplay plugins]
    AppMode --> Presentation[Client presentation]
    Network --> Gameplay
    Gameplay --> Presentation
    Presentation --> Renderer[Renderer]
    Presentation --> UI[UI]
    Presentation --> Scenes[Scenes]
```

### Application Roles

`network::mode::AppMode` is the source of truth for whether the process has server and/or client responsibilities.

| Mode | Server systems | Client presentation |
| --- | ---: | ---: |
| `Client` | No | Yes |
| `Server` | Yes | No |
| `HostClient` | Yes | Yes |

Systems should be gated with:

- `network::mode::has_server` for server-authoritative logic.
- `network::mode::has_client` for client-side presentation/input/UI logic.

Do not infer application role from transport configuration presence. In `HostClient` mode, both client and server configuration can exist intentionally.

### Server Responsibilities

The server is headless and authoritative. It owns gameplay simulation, persistence, migration execution, and replicated state production. It does not register scenes, UI, or rendering plugins.

### Client Responsibilities

The client reads replicated gameplay state and creates local presentation state such as:

- `Mesh3d`, materials, and `Transform` components.
- UI widgets and HUD state.
- Local visual indicators for movement and targeting.

This separation keeps simulation state portable and avoids coupling authoritative gameplay with presentation details.

## Controls

Default controls are defined in `crates/client/src/input/key_mapping.rs` and related input systems.

| Input | Action |
| --- | --- |
| Right mouse button | Move to world position |
| Left mouse button | Select target |
| `Space` | Cast basic attack |
| `Q` | Cast ray of light |
| `E` | Cast fireball |
| `R` | Cast healing circle |
| `T` | Cast meteorite |
| `F` | Cast swift |
| `Tab` | Show scoreboard |
| `Escape` | Toggle pause / clear target depending on current UI state |

## Adding Gameplay Content

Gameplay entities are organized under `crates/shared/src/entity/`. A typical entity module contains:

```text
crates/shared/src/entity/<name>/
├── mod.rs
├── components.rs
├── spawn.rs
└── systems.rs
```

To add a replicated entity:

1. Create the entity submodule.
2. Define a marker component.
3. Implement `EntityDefinition`.
4. Register systems in the entity plugin.
5. Register the plugin in `crates/shared/src/entity/mod.rs`.
6. Spawn it through the shared entity spawn helpers where appropriate.

See [`docs/create-a-new-plugin.md`](docs/create-a-new-plugin.md) for the detailed guide.

## Troubleshooting

### `DATABASE_URL is required when starting a server`

Server and host-client modes require a PostgreSQL connection string. Provide it through one of these options:

- `config/development.toml`
- `config/local.toml`
- `DATABASE_URL` environment variable

Example:

```sh
DATABASE_URL=postgresql://bevy:bevy@127.0.0.1:5433/bevy_mmo cargo run -- server
```

### Client cannot connect to server

Check that:

- The server is running.
- UDP port `5051` is reachable.
- The client uses the correct `client.server_addr` or `--server-addr`.
- Firewalls or container/network rules are not blocking UDP traffic.

### Duplicate client identity issues

The client automatically generates a non-zero Netcode client ID when `--client-id` is omitted. If you pass `--client-id` manually, make sure every concurrent local client uses a different non-zero value.

### Client mode fails in a server-only build

Client and host-client modes require the `client` Cargo feature. If you built with `--no-default-features --features server,...`, run only server mode or rebuild with client features enabled.

## Further Documentation

- [`docs/architecture.md`](docs/architecture.md): deeper architecture notes and module boundaries.
- [`docs/database.md`](docs/database.md): PostgreSQL, Docker, and migration details.
- [`docs/create-a-new-plugin.md`](docs/create-a-new-plugin.md): entity plugin creation guide.
- [`plans/`](plans/): implementation plans and design notes for upcoming or recent features.

## Contributing

Contributions are welcome. For a smooth contribution flow:

1. Open an issue or discussion for larger gameplay, networking, or persistence changes.
2. Keep changes focused and consistent with the plugin-based architecture.
3. Run formatting, tests, and Clippy before opening a pull request:

```sh
cargo fmt
cargo test
cargo clippy -- -D warnings
```

When adding gameplay systems, make sure server and client responsibilities are gated with the correct run conditions described in [Architecture Overview](#architecture-overview).

## Security and Secrets

This repository is intended to be public. Do not commit secrets, production credentials, or private connection strings.

Use one of these mechanisms instead:

- `.env` for local Docker Compose variables.
- `config/local.toml` for local machine overrides.
- Environment variables for production deployment.

The template files `.env.example` and `config/local.toml.example` should contain only safe example values.

## License

This project is licensed under the [MIT License](LICENSE).
