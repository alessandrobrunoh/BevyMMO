# =============================================================================
# Stage 1 — Dependency cache
# Pre-compila solo le dipendenze sfruttando la cache layer di Docker.
# Questo layer viene ricostruito solo se Cargo.toml / Cargo.lock cambiano.
# =============================================================================
FROM rust:1-slim-bookworm AS deps

# Installa le dipendenze di sistema necessarie per compilare.
# Il server necessita solo di OpenSSL e libpq per il database.
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copia solo i manifest per sfruttare il layer caching di Docker:
# se il codice sorgente cambia ma Cargo.toml/lock restano uguali,
# questo stage viene servito dalla cache senza ricompilare le dipendenze.
COPY Cargo.toml Cargo.lock ./

# Crea un main.rs stub così cargo può compilare le dipendenze in isolamento
RUN mkdir src && echo 'fn main() {}' > src/main.rs

# Compila solo le dipendenze in modalità server (no client, no UI, no renderer)
RUN cargo build --release \
    --no-default-features \
    --features server,netcode,udp,replication \
    --bin game \
    && rm -rf src

# =============================================================================
# Stage 2 — Builder
# Compila il sorgente reale usando la cache delle dipendenze dello stage 1.
# =============================================================================
FROM deps AS builder

# Copia il sorgente reale
COPY src ./src

# Invalida il timestamp del binario stub per forzare la ricompilazione del bin
RUN touch src/main.rs

# Build release finale — solo modalità server
RUN cargo build --release \
    --no-default-features \
    --features server,netcode,udp,replication \
    --bin game

# Strip dei simboli di debug per ridurre drasticamente le dimensioni del binario
RUN strip target/release/game

# =============================================================================
# Stage 3 — Runtime (immagine finale minima)
# Usa Debian slim: solo le librerie dinamiche strettamente necessarie.
# Nessun compilatore, nessun toolchain Rust -> immagine finale ~50-80 MB.
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Etichette OCI standard per il registry GitHub
LABEL org.opencontainers.image.source="https://github.com/alessandrobrunoh/BevyMMO"
LABEL org.opencontainers.image.description="BevyMMO dedicated game server"
LABEL org.opencontainers.image.licenses="MIT"

# Dipendenze runtime minime (libssl + libpq per SeaORM/PostgreSQL)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Utente non-root per principio del minimo privilegio
RUN useradd --uid 1001 --no-create-home --shell /sbin/nologin server
USER server

WORKDIR /app

# Copia solo il binario strippato dallo stage builder
COPY --from=builder /build/target/release/game ./game

# Porta UDP usata da Lightyear/Netcode per i client
EXPOSE 5051/udp

# Avvio diretto in modalità server (senza shell per evitare PID 1 wrapping)
ENTRYPOINT ["./game", "server"]
