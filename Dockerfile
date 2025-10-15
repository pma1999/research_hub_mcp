# Stage 1: Chef planner
FROM rust:1.74-slim AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Stage 2: Recipe preparation
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY build.rs ./
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Build dependencies (CACHED LAYER)
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies only - this layer is cached
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates && \
    rm -rf /var/lib/apt/lists/*
RUN cargo chef cook --profile docker --recipe-path recipe.json

# Stage 4: Build application
COPY . .
# Use faster docker profile for compilation
RUN cargo build --profile docker

# Stage 5: Runtime with Node.js wrapper
FROM node:20-slim
RUN useradd -m -u 10001 mcp && \
    apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /data && chown -R mcp:mcp /data

WORKDIR /home/mcp

# Copy binary from builder (docker profile outputs to target/docker/)
COPY --from=builder /app/target/docker/rust-research-mcp /usr/local/bin/rust-research-mcp

# Copy HTTP wrapper
COPY mcp-http-wrapper.js package.json ./
RUN chown -R mcp:mcp /home/mcp && chmod +x mcp-http-wrapper.js

USER mcp

# Environment variables
ENV RUST_LOG=info \
    PORT=3000 \
    DOWNLOAD_DIR=/data \
    LOG_LEVEL=info

EXPOSE 3000

# Run the HTTP wrapper (which spawns the Rust binary)
CMD ["node", "mcp-http-wrapper.js"]

