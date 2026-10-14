#!/bin/bash
# Build and start Synapse.
#   ./start_rust_server.sh          -> MCP over HTTP on $SYNAPSE_MCP_PORT (default 3000)
#   ./start_rust_server.sh --stdio  -> MCP over stdin/stdout
set -euo pipefail

cd "$(dirname "$0")"

if ! command -v cargo &> /dev/null; then
    if [ -f "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$PATH:$HOME/.cargo/bin"
    else
        echo "cargo not found. Install Rust: https://rustup.rs" >&2
        exit 1
    fi
fi

# The MCP App UI (ui:// resources) is served from frontend/dist/index.html.
if command -v npm &> /dev/null; then
    echo "Building frontend MCP App..." >&2
    (cd frontend && npm install && npm run build) >&2
else
    echo "npm not found: skipping frontend build (ui:// resources will show an error page)" >&2
fi

echo "Building Rust server..." >&2
cargo build --release -p synapse-core >&2

exec ./target/release/synapse "$@"
