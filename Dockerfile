FROM rust:1.88-alpine as builder

# Add edge repositories for onnxruntime and its dependencies (abseil-cpp, protobuf)
RUN echo "http://dl-cdn.alpinelinux.org/alpine/edge/main" >> /etc/apk/repositories && \
    echo "http://dl-cdn.alpinelinux.org/alpine/edge/community" >> /etc/apk/repositories

# Install build dependencies
# - musl-dev, gcc, g++, make: Standard build tools
# - perl: Required for OpenSSL build scripts (if building from source)
# - protobuf: Required for prost-build (protoc)
# - openssl-dev: Required for linking against system OpenSSL (dynamic)
# - onnxruntime-dev: Required for linking against system ONNX Runtime (dynamic)
# - abseil-cpp-dev: Required by onnxruntime-dev
# - protobuf-dev: Required by onnxruntime-dev (for libprotobuf-lite)
# - pkgconfig: Required to find system libraries
# - clang, clang-dev: Required by some crates
# - clang-static: Required for bindgen on musl (dynamic loading not supported)
# - git: Required by cargo
RUN apk update && apk add --no-cache \
    musl-dev \
    perl \
    make \
    gcc \
    g++ \
    git \
    protobuf \
    openssl-dev \
    onnxruntime-dev \
    abseil-cpp-dev \
    protobuf-dev \
    pkgconfig \
    clang \
    clang-dev \
    clang-static

WORKDIR /usr/src/synapse
COPY . .

# Set environment variables
# OPENSSL_NO_VENDOR=1: Use system OpenSSL (dynamic)
# ORT_STRATEGY=system: Use system ONNX Runtime (dynamic)
# LIBCLANG_PATH: Explicitly point to the directory containing libclang.a/so
# LIBCLANG_STATIC_PATH: Force bindgen to link statically against libclang
ENV OPENSSL_NO_VENDOR=1
ENV ORT_STRATEGY=system
ENV LIBCLANG_PATH=/usr/lib
ENV LIBCLANG_STATIC_PATH=/usr/lib

# Build the binary
# Note: The resulting binary will be dynamically linked against musl, openssl, onnxruntime, abseil, and protobuf
# This is required because onnxruntime static binaries are not available for musl
RUN cargo build --release -p synapse-core

# Final stage
FROM alpine:edge

# Add edge repositories for runtime dependencies
RUN echo "http://dl-cdn.alpinelinux.org/alpine/edge/main" >> /etc/apk/repositories && \
    echo "http://dl-cdn.alpinelinux.org/alpine/edge/community" >> /etc/apk/repositories

# Install runtime dependencies
RUN apk add --no-cache \
    libgcc \
    libstdc++ \
    openssl \
    onnxruntime \
    abseil-cpp \
    protobuf \
    ca-certificates

# Copy the binary from the builder stage
COPY --from=builder /usr/src/synapse/target/release/synapse /usr/local/bin/synapse

ENV GRAPH_STORAGE_PATH=/data/graphs
VOLUME /data/graphs
EXPOSE 50051

ENTRYPOINT ["synapse"]
