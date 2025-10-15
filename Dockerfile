# Etapa de build
FROM rust:1.74-slim AS builder
WORKDIR /app
# Paquetes frecuentes que necesita cargo (openssl, etc.)
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src
COPY benches ./benches
# Compila en release
RUN cargo build --release

# Etapa de runtime con Node.js
FROM node:20-slim
RUN useradd -m -u 10001 mcp \
  && apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 \
  && rm -rf /var/lib/apt/lists/* \
  && mkdir -p /data && chown -R mcp:mcp /data

WORKDIR /home/mcp

# Copiar el binario Rust
COPY --from=builder /app/target/release/rust-research-mcp /usr/local/bin/rust-research-mcp

# Copiar wrapper HTTP y package.json
COPY mcp-http-wrapper.js package.json ./

# Cambiar permisos
RUN chown -R mcp:mcp /home/mcp && chmod +x mcp-http-wrapper.js

USER mcp

# Variables de entorno
ENV RUST_LOG=info
ENV PORT=3000
ENV DOWNLOAD_DIR=/data
ENV LOG_LEVEL=info

# Exponer puerto
EXPOSE 3000

# Ejecutar el wrapper HTTP (que luego ejecuta el binario Rust)
CMD ["node", "mcp-http-wrapper.js"]

