# Build stage
FROM rust:1-slim-bookworm AS builder

ARG GATEWAY_REPOSITORY_URL="https://github.com/alessandrobrunoh/BevyMMO.git"
ARG GATEWAY_REPOSITORY_REF="main"

WORKDIR /usr/src/app

# Install compilation prerequisites. `git` is only used when a deploy tool sends
# `apps/gateway` as the Docker context instead of the workspace root.
RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Accept both supported build contexts:
# - repository root: used by docker-compose and local workspace builds;
# - apps/gateway: used by some deploy tools when the resource path is nested.
COPY . /tmp/build-context/
RUN if [ -f /tmp/build-context/Cargo.toml ] && [ -d /tmp/build-context/crates ]; then \
        cp -a /tmp/build-context/. /usr/src/app/; \
    else \
        git clone --depth 1 --branch "${GATEWAY_REPOSITORY_REF}" "${GATEWAY_REPOSITORY_URL}" /usr/src/app; \
        rm -rf /usr/src/app/apps/gateway; \
        mkdir -p /usr/src/app/apps/gateway; \
        cp -a /tmp/build-context/. /usr/src/app/apps/gateway/; \
    fi

# Build gateway binary in release mode
RUN cargo build --release --package bevymmo_gateway

# Runtime stage
FROM debian:bookworm-slim AS runtime

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy configuration files
COPY config/ ./config/

# Copy compiled binary from builder stage
COPY --from=builder /usr/src/app/target/release/gateway /app/gateway

ENV APP_ENV=production
ENV GATEWAY__BIND_ADDR=0.0.0.0:8001
ENV GATEWAY__LOG_FORMAT=json

EXPOSE 8001

ENTRYPOINT ["/app/gateway"]
