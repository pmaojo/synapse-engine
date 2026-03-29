#!/bin/bash
# Start the Synapse Semantic Engine (ex-Grafoso)

MODE="grpc"
if [[ "$1" == "--mcp" ]]; then
    MODE="mcp"
fi

echo "🚀 Starting Synapse ($MODE mode)..."
echo ""

# Check if cargo is in path
if ! command -v cargo &> /dev/null; then
    # Try common local path
    if [ -f "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$PATH:$HOME/.cargo/bin"
    else
        echo "❌ cargo not found. Please install Rust: https://rustup.rs"
        exit 1
    fi
fi

cd "$(dirname "$0")/crates/semantic-engine"

echo "📦 Building Frontend Ext-App..."
cd ../../frontend
npm install
npm run build
cd ../crates/semantic-engine

# Build the server
echo "📦 Building Rust server..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    echo ""
    if [[ "$MODE" == "mcp" ]]; then
        echo "🔌 Running in MCP mode (stdio)..."
        ../../target/release/synapse --mcp
    else
        echo "🌐 Starting gRPC server on localhost:50051..."
        ../../target/release/synapse
    fi
else
    echo "❌ Build failed. Check errors above."
    exit 1
fi
