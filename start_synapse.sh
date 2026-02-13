#!/bin/bash
# Start Synapse gRPC server with authentication token

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Prefer target/release if available, otherwise bin/synapse
if [ -f "$SCRIPT_DIR/target/release/synapse" ]; then
    SYNAPSE_BIN="$SCRIPT_DIR/target/release/synapse"
else
    SYNAPSE_BIN="$SCRIPT_DIR/bin/synapse"
fi
LOG_FILE="$SCRIPT_DIR/synapse_grpc.log"
PID_FILE="$SCRIPT_DIR/synapse.pid"
STORAGE_PATH="$SCRIPT_DIR/data/graphs"
AUTH_TOKENS='{"test": ["*"]}'
MOCK_EMBEDDINGS=${MOCK_EMBEDDINGS:-"true"}

# Check if synapse binary exists
if [ ! -f "$SYNAPSE_BIN" ]; then
    echo "Error: synapse binary not found at $SYNAPSE_BIN"
    exit 1
fi

# Check if already running
if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if kill -0 "$PID" 2>/dev/null; then
        echo "Synapse already running (PID $PID)"
        exit 0
    else
        echo "Stale PID file, removing..."
        rm "$PID_FILE"
    fi
fi

# Ensure storage directory exists
mkdir -p "$STORAGE_PATH"

# Start synapse
export SYNAPSE_AUTH_TOKENS="$AUTH_TOKENS"
export MOCK_EMBEDDINGS="$MOCK_EMBEDDINGS"
nohup "$SYNAPSE_BIN" > "$LOG_FILE" 2>&1 &
SYNAPSE_PID=$!

# Wait a bit and check if process is still alive
sleep 2
if kill -0 "$SYNAPSE_PID" 2>/dev/null; then
    echo "$SYNAPSE_PID" > "$PID_FILE"
    echo "Synapse started (PID $SYNAPSE_PID)"
    echo "Logs: $LOG_FILE"
    echo "Storage: $STORAGE_PATH"
    echo "gRPC endpoint: localhost:50051"
else
    echo "Failed to start Synapse. Check logs: $LOG_FILE"
    exit 1
fi