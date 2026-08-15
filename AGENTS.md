# BevyMMO Agent Guide

This document contains high-signal, repo-specific facts to help agents avoid mistakes and understand non-obvious conventions in this repository.

## Commands & Workflows

**Building & Running**
The repository is now a Cargo workspace. The runnable binary lives in `bins/game`, and the workspace root sets `default-members = ["bins/game"]`, so the classic commands still work from the repository root.

- **Run Client**: `cargo run -- client`
- **Run Server**: `cargo run -- server` (Requires PostgreSQL, see Database section)
- **Run Host-Client**: `cargo run -- host-client` (Embedded server + client)
- **Build Dedicated Production Server**: 
  `cargo build --release --no-default-features --features server,netcode,udp,replication --bin game`
  *(This skips building client UI/rendering for a much smaller footprint).*

**Testing & Verification**
- **Test**: `cargo test`
- **Lint**: `cargo clippy -- -D warnings`

## Database & Environment

The server requires a PostgreSQL connection to run.

- **Local Setup**: 
  1. `cp .env.example .env`
  2. `docker compose up -d`
- **Migrations**: SeaORM migrations are applied **automatically** by the server at startup. There is no manual `sea-orm-cli` step required to run existing migrations. (To add a new migration, place it in `src/migrations/`).
- **Configuration**: Uses layered config: `config/default.toml` <- `config/<APP_ENV>.toml` <- `config/local.toml` <- ENV VARS. 
  - `APP_ENV` defaults to `development`.
  - The `DATABASE_URL` is required to run the server, and is provided out-of-the-box for local dev in `config/development.toml`. Do not commit secrets; place them in `config/local.toml` (gitignored).

## Architecture & Code Conventions

- **Workspace split**:
  - `bevymmo_shared` = pure shared data/contracts only
  - `bevymmo_server` = authoritative runtime + persistence
  - `bevymmo_client` = client-only runtime helpers
  - `bevymmo_presentation` = rendering, scenes, UI
  - `bins/game` = composition root CLI binary

- **Shared means data only**: do not put sockets, rendering, or Bevy UI inside `bevymmo_shared`.

- **Application Roles (CRITICAL)**: System logic must be gated using the Bevy run conditions `network::mode::has_server` and `network::mode::has_client`. Do **not** infer the application role simply by checking if a client/server transport config exists, because in `HostClient` mode, both exist simultaneously.
- **Headless Server**: The server is purely headless and does not register scenes, UI, or rendering plugins.
- **Client Presentation**: The client's renderer creates visual representation by reading the replicated state and adding local components (like `Mesh3d`, materials, and `Transform`). UI widgets similarly build local views of replicated gameplay state.
- **Docs**: Read `docs/architecture.md`, `docs/database.md`, and `docs/create-a-new-plugin.md` for deeper structural guidelines.
