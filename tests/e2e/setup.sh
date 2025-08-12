#!/bin/bash
# Setup script for Trabas E2E tests
# This script sets up the test environment and starts all necessary services

set -e  # Exit on any error

# Configuration
WORKSPACE_DIR="${WORKSPACE_DIR:-/tmp/trabas_test}"
TRABAS_BINARY="${TRABAS_BINARY:-./target/release/trabas}"
# Mock server ports
MOCK_SERVER_PORT="${MOCK_SERVER_PORT:-3000}"
PUBLIC_PORT="${PUBLIC_PORT:-8001}"
CLIENT_PORT="${CLIENT_PORT:-8002}"
# Mock server TLS ports (for TLS tunnel test)
MOCK_SERVER_TLS_PORT="${MOCK_SERVER_TLS_PORT:-3001}"
PUBLIC_PORT_TLS="${PUBLIC_PORT_TLS:-8003}"
CLIENT_PORT_TLS="${CLIENT_PORT_TLS:-8004}"

CLIENT_ID="${CLIENT_ID:-e2e-test-client}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[E2E Setup]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[E2E Setup]${NC} $1"
}

error() {
    echo -e "${RED}[E2E Setup]${NC} $1"
}

# Create workspace directory
log "Creating workspace directory: $WORKSPACE_DIR"
mkdir -p "$WORKSPACE_DIR"

# Verify binary exists
if [ ! -f "$TRABAS_BINARY" ]; then
    error "Trabas binary not found at: $TRABAS_BINARY"
    error "Please build the project first: cargo build --release --manifest-path cli/Cargo.toml"
    exit 1
fi

log "Using Trabas binary: $TRABAS_BINARY"

# Make binary executable
chmod +x "$TRABAS_BINARY"

# Get the directory where the binary is located (this is where trabas_config will be created)
BINARY_DIR=$(dirname "$TRABAS_BINARY")
CONFIG_DIR="$BINARY_DIR/trabas_config"
CONFIG_FILE="$CONFIG_DIR/.env"

# Get the project root directory (where the tests/e2e directory is located)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MOCK_SERVER_SCRIPT="$SCRIPT_DIR/mock_server.py"

log "Config will be created at: $CONFIG_DIR"
log "Project root: $PROJECT_ROOT"
log "Mock server script: $MOCK_SERVER_SCRIPT"

# Verify mock server script exists
if [ ! -f "$MOCK_SERVER_SCRIPT" ]; then
    error "Mock server script not found at: $MOCK_SERVER_SCRIPT"
    exit 1
fi

# Change to workspace directory for running commands
cd "$WORKSPACE_DIR"

# Generate server configuration
log "Generating server configuration..."
"$TRABAS_BINARY" server set-config \
    --gen-key \
    --public-endpoint "http://localhost:$PUBLIC_PORT" \
    --redis-enable false \
    --force

# Generate SSL Keys (for TLS tunnel)
log "Generating SSL keys..."
"$TRABAS_BINARY" server ssl-config generate-keys --host localhost --ip 127.0.0.1 --force || true

# Set GLOBAL_DEBUG to true in the config file
echo "GLOBAL_DEBUG=true" >> "$CONFIG_FILE"

# Extract the generated secret from the correct location
if [ ! -f "$CONFIG_FILE" ]; then
    error "Config file not found at: $CONFIG_FILE"
    error "Available files in config directory:"
    ls -la "$CONFIG_DIR" 2>/dev/null || echo "Config directory does not exist"
    exit 1
fi

SERVER_SECRET=$(cat "$CONFIG_FILE" | grep SV_SECRET | cut -d'=' -f2)
if [ -z "$SERVER_SECRET" ]; then
    error "Failed to extract server secret from: $CONFIG_FILE"
    error "Config file contents:"
    cat "$CONFIG_FILE"
    exit 1
fi

log "Generated server secret: ${SERVER_SECRET:0:10}..."

# Generate client configuration
log "Generating client configuration..."
"$TRABAS_BINARY" client set-config \
    --client-id "$CLIENT_ID" \
    --server-host localhost \
    --server-port "$CLIENT_PORT" \
    --server-signing-key "$SERVER_SECRET" \
    --force

# Show generated configuration
log "Generated configuration:"
cat "$CONFIG_FILE"

########################################
# Start mock servers (plain + TLS target)
########################################
log "Starting mock server on port $MOCK_SERVER_PORT..."
python3 "$MOCK_SERVER_SCRIPT" --port "$MOCK_SERVER_PORT" &
MOCK_PID=$!
echo $MOCK_PID > /tmp/mock_server.pid

# Wait for mock server to start
sleep 2

# Verify mock server is running
if ! curl -f "http://localhost:$MOCK_SERVER_PORT/ping" >/dev/null 2>&1; then
    error "Mock server failed to start"
    kill $MOCK_PID 2>/dev/null || true
    exit 1
fi

log "Mock server started successfully (PID: $MOCK_PID)"

# Start second mock server for TLS tunnel
log "Starting mock server (TLS target) on port $MOCK_SERVER_TLS_PORT..."
python3 "$MOCK_SERVER_SCRIPT" --port "$MOCK_SERVER_TLS_PORT" &
MOCK_TLS_PID=$!
echo $MOCK_TLS_PID > /tmp/mock_server_tls.pid

# Wait for TLS mock server to start
sleep 2

# Verify TLS target mock server is running
if ! curl -f "http://localhost:$MOCK_SERVER_TLS_PORT/ping" >/dev/null 2>&1; then
    error "TLS target mock server failed to start"
    kill $MOCK_PID 2>/dev/null || true
    kill $MOCK_TLS_PID 2>/dev/null || true
    exit 1
fi

log "TLS target mock server started successfully (PID: $MOCK_TLS_PID)"

# Start Trabas server
log "Starting Trabas server (public: $PUBLIC_PORT, client: $CLIENT_PORT)..."
"$TRABAS_BINARY" server run --public-port "$PUBLIC_PORT" --client-port "$CLIENT_PORT" &
SERVER_PID=$!
echo $SERVER_PID > /tmp/trabas_server.pid

# Wait for server to start
sleep 3

# Verify server is running
if ! ps -p $SERVER_PID > /dev/null; then
    error "Trabas server failed to start"
    kill $MOCK_PID 2>/dev/null || true
    exit 1
fi

log "Trabas server started successfully (PID: $SERVER_PID)"

########################################
# Start Trabas server with TLS (client listener over TLS)
########################################
log "Starting Trabas server with TLS (public: $PUBLIC_PORT_TLS, client: $CLIENT_PORT_TLS)..."
"$TRABAS_BINARY" server run --public-port "$PUBLIC_PORT_TLS" --client-port "$CLIENT_PORT_TLS" --tls &
SERVER_TLS_PID=$!
echo $SERVER_TLS_PID > /tmp/trabas_server_tls.pid

# Wait for server to start
sleep 3

# Verify server is running
if ! ps -p $SERVER_TLS_PID > /dev/null; then
    error "Trabas server with TLS failed to start"
    kill $MOCK_PID 2>/dev/null || true
    kill $MOCK_TLS_PID 2>/dev/null || true
    exit 1
fi

log "Trabas server with TLS started successfully (PID: $SERVER_TLS_PID)"

# Start Trabas client
log "Starting Trabas client..."
"$TRABAS_BINARY" client serve --host localhost --port "$MOCK_SERVER_PORT" &
CLIENT_PID=$!
echo $CLIENT_PID > /tmp/trabas_client.pid

# Wait for client to connect
sleep 5

# Verify client is running
if ! ps -p $CLIENT_PID > /dev/null; then
    error "Trabas client failed to start"
    kill $SERVER_PID 2>/dev/null || true
    kill $MOCK_PID 2>/dev/null || true
    exit 1
fi

log "Trabas client started successfully (PID: $CLIENT_PID)"

########################################
# Start Trabas client with TLS
########################################
log "Starting Trabas client with TLS..."
# Override CL_SERVER_PORT just for this process to point to TLS client port
CL_SERVER_PORT="$CLIENT_PORT_TLS" "$TRABAS_BINARY" client serve --host localhost --port "$MOCK_SERVER_TLS_PORT" --tls &
CLIENT_TLS_PID=$!
echo $CLIENT_TLS_PID > /tmp/trabas_client_tls.pid

# Wait for client to connect
sleep 5

# Verify client is running
if ! ps -p $CLIENT_TLS_PID > /dev/null; then
    error "Trabas client with TLS failed to start"
    kill $SERVER_TLS_PID 2>/dev/null || true
    kill $MOCK_PID 2>/dev/null || true
    kill $MOCK_TLS_PID 2>/dev/null || true
    exit 1
fi

log "Trabas client with TLS started successfully (PID: $CLIENT_TLS_PID)"

# Test basic connectivity
log "Testing basic connectivity..."
if curl -f "http://localhost:$PUBLIC_PORT/$CLIENT_ID/ping" >/dev/null 2>&1; then
    log "Basic connectivity test passed!"
else
    warn "Basic connectivity test failed, but continuing with full tests..."
fi


log "Testing basic connectivity (TLS tunnel)..."
if curl -f "http://localhost:$PUBLIC_PORT_TLS/$CLIENT_ID/ping" >/dev/null 2>&1; then
    log "Basic connectivity test passed!"
else
    warn "Basic connectivity test failed, but continuing with full tests..."
fi

log "Setup complete! Ready to run E2E tests."
log "Services running:"
log "  Mock server:    http://localhost:$MOCK_SERVER_PORT (PID: $MOCK_PID)"
log "  Trabas server:  http://localhost:$PUBLIC_PORT (PID: $SERVER_PID)"
log "  Trabas client:  PID: $CLIENT_PID"
log "  TLS mock server: http://localhost:$MOCK_SERVER_TLS_PORT (PID: $MOCK_TLS_PID)"
log "  Trabas server (TLS):  http://localhost:$PUBLIC_PORT_TLS (PID: $SERVER_TLS_PID)"
log "  Trabas client (TLS):  PID: $CLIENT_TLS_PID"
log ""
log "To run tests: python3 $SCRIPT_DIR/run_tests.py --server-url http://localhost:$PUBLIC_PORT --client-id $CLIENT_ID"
log "To cleanup: $SCRIPT_DIR/cleanup.sh"
