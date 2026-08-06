# =============================================================================
# Stage 1 — Dependency cache
# Pre-compiles only dependencies leveraging Docker layer cache.
# This layer is rebuilt only if Cargo.toml / Cargo.lock change.
# =============================================================================
FROM rust:1-slim-bookworm AS deps

# Install required system dependencies for compilation.
# Server requires only OpenSSL and libpq for the database.
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy only manifests to leverage Docker layer caching:
# if source code changes but Cargo.toml/lock remain identical,
# this stage is served from cache without recompiling dependencies.
COPY Cargo.toml Cargo.lock ./

# Create a stub main.rs so cargo can compile dependencies in isolation
RUN mkdir src && echo 'fn main() {}' > src/main.rs

# Compile only dependencies in server mode (no client, no UI, no renderer)
RUN cargo build --release \
    --no-default-features \
    --features server,netcode,udp,replication \
    --bin game \
    && rm -rf src

# =============================================================================
# Stage 2 — Builder
# Compiles real source using dependency cache from Stage 1.
# =============================================================================
FROM deps AS builder

# Copy real source
COPY src ./src

# Copy configuration files: required at compile time
# (`include_str!("../config/default.toml")` in `settings.rs`) and at runtime
# (`Settings::load` reads `config/<env>.toml` and `config/local.toml`).
COPY config ./config

# Invalidate timestamp of stub binary to force bin recompilation
RUN touch src/main.rs

# Final release build — server mode only
RUN cargo build --release \
    --no-default-features \
    --features server,netcode,udp,replication \
    --bin game

# Strip debug symbols to drastically reduce binary size
RUN strip target/release/game

# =============================================================================
# Stage 3 — Runtime (minimal final image)
# Uses Debian slim: only strictly necessary dynamic libraries.
# No compiler, no Rust toolchain -> final image ~50-80 MB.
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Standard OCI labels for GitHub registry
LABEL org.opencontainers.image.source="https://github.com/alessandrobrunoh/BevyMMO"
LABEL org.opencontainers.image.description="BevyMMO dedicated game server"
LABEL org.opencontainers.image.licenses="MIT"

# Minimal runtime dependencies (libssl + libpq for SeaORM/PostgreSQL)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Non-root user for principle of least privilege
RUN useradd --uid 1001 --no-create-home --shell /sbin/nologin server
USER server

WORKDIR /app

# Copy stripped binary from builder stage
COPY --from=builder /build/target/release/game ./game

# Copy configuration files read at runtime by `Settings::load`.
COPY --from=builder /build/config ./config

# UDP port used by Lightyear/Netcode for clients
EXPOSE 5051/udp

# Direct startup in server mode (no shell to avoid PID 1 wrapping)
ENTRYPOINT ["./game", "server"]
