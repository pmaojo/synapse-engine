FROM rust:1.88-alpine as builder

# Install build dependencies
# - musl-dev, gcc, g++, make: Standard build tools
# - perl: Required for OpenSSL build scripts (if building from source)
# - protobuf: Required for prost-build (protoc)
# - openssl-dev, openssl-libs-static: Required for linking against system OpenSSL (static)
# - pkgconfig: Required to find system libraries
# - clang, clang-dev: Required by some crates (e.g. rocksdb/bindgen)
# - git: Required by cargo to fetch git dependencies
RUN apk add --no-cache \
    musl-dev \
    perl \
    make \
    gcc \
    g++ \
    git \
    protobuf \
    openssl-dev \
    openssl-libs-static \
    pkgconfig \
    clang \
    clang-dev

WORKDIR /usr/src/synapse
COPY . .

# Set environment variables for static build
# OPENSSL_NO_VENDOR=1: Use system OpenSSL instead of building from source
# OPENSSL_STATIC=1: Link against static OpenSSL libraries
# RUSTFLAGS="-C target-feature=-crt-static": Not needed - musl targets use static CRT by default
ENV OPENSSL_NO_VENDOR=1
ENV OPENSSL_STATIC=1

# Build the binary
# Default target for rust:alpine is musl, so no --target needed for native arch
# This works for both amd64 and arm64 (if multi-arch image is used)
RUN cargo build --release -p synapse-core

# Final stage
FROM alpine:latest

# Install CA certificates for HTTPS support
RUN apk add --no-cache ca-certificates

# Copy the binary from the builder stage
COPY --from=builder /usr/src/synapse/target/release/synapse /usr/local/bin/synapse

ENV GRAPH_STORAGE_PATH=/data/graphs
VOLUME /data/graphs
EXPOSE 50051

ENTRYPOINT ["synapse"]
