# Build stage
FROM rust:1-slim-bookworm AS builder

WORKDIR /usr/src/app

# Install compilation prerequisites
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo workspace configuration and source trees
COPY Cargo.toml Cargo.lock ./
COPY config/ ./config/
COPY crates/ ./crates/
COPY apps/ ./apps/
COPY bins/ ./bins/

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
