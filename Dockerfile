# Etapa de build
FROM rust:1.74-slim AS builder
WORKDIR /app
# Paquetes frecuentes que necesita cargo (openssl, etc.)
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY . .
# Compila en release
RUN cargo build --release

# Etapa de runtime mínima
FROM debian:bookworm-slim
RUN useradd -m -u 10001 mcp \
  && apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 \
  && rm -rf /var/lib/apt/lists/* \
  && mkdir -p /data && chown -R mcp:mcp /data
USER mcp
WORKDIR /home/mcp
# Copiamos el binario
COPY --from=builder /app/target/release/rust-research-mcp /usr/local/bin/rust-research-mcp
ENV RUST_LOG=info
# Smithery conecta por stdio; solo necesitamos ejecutar el binario
CMD ["/usr/local/bin/rust-research-mcp","--download-dir","/data","--log-level","info"]

