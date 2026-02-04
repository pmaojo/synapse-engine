FROM rust:1.85-slim as builder
RUN apt-get update && apt-get install -y protobuf-compiler pkg-config libssl-dev clang libclang-dev
WORKDIR /usr/src/synapse
COPY . .
RUN cargo build --release -p synapse-core

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/synapse/target/release/synapse /usr/local/bin/synapse
ENV GRAPH_STORAGE_PATH=/data/graphs
VOLUME /data/graphs
EXPOSE 50051
ENTRYPOINT ["synapse"]
