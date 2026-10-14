FROM node:22-bookworm-slim AS frontend

WORKDIR /usr/src/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

FROM rust:1-bookworm AS builder

# clang/libclang are needed by bindgen when building RocksDB (oxrocksdb-sys)
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/synapse
COPY . .

RUN cargo build --release -p synapse-core

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/synapse/target/release/synapse /usr/local/bin/synapse
# MCP App UI served for ui:// resources (looked up at /app/frontend/dist/index.html)
COPY --from=frontend /usr/src/frontend/dist /app/frontend/dist
WORKDIR /app

ENV GRAPH_STORAGE_PATH=/data/graphs
VOLUME /data/graphs
EXPOSE 3000

ENTRYPOINT ["synapse"]
