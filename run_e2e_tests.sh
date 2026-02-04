#!/bin/bash
# Grafoso E2E Test Runner
# Starts Rust backend and runs comprehensive E2E tests

set -e  # Exit on error

echo "🚀 Grafoso E2E Test Suite"
echo "=========================="
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if Rust backend is already running
if lsof -Pi :50051 -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Rust backend already running on port 50051${NC}"
    echo "Using existing instance..."
    RUST_RUNNING=true
else
    echo "📦 Building Rust backend..."
    cd crates/semantic-engine
    cargo build --release
    cd ../..
    
    echo ""
    echo "🔧 Starting Rust backend..."
    ./target/release/semantic-engine &
    RUST_PID=$!
    RUST_RUNNING=false
    
    # Wait for Rust to be ready
    echo "⏳ Waiting for Rust backend to start..."
    sleep 3
    
    # Check if it's running
    if ! kill -0 $RUST_PID 2>/dev/null; then
        echo -e "${RED}❌ Failed to start Rust backend${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✅ Rust backend started (PID: $RUST_PID)${NC}"
fi

echo ""
echo "🧪 Running E2E Tests..."
echo "----------------------"

# Run tests
if python -m pytest tests/test_e2e.py tests/test_hexagonal.py -v --tb=short; then
    echo ""
    echo -e "${GREEN}✅ All tests passed!${NC}"
    TEST_EXIT=0
else
    echo ""
    echo -e "${RED}❌ Some tests failed${NC}"
    TEST_EXIT=1
fi

# Cleanup
if [ "$RUST_RUNNING" = false ]; then
    echo ""
    echo "🧹 Stopping Rust backend..."
    kill $RUST_PID 2>/dev/null || true
    echo -e "${GREEN}✅ Cleanup complete${NC}"
fi

echo ""
echo "=========================="
echo "Test run complete!"

exit $TEST_EXIT
