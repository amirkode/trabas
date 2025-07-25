#!/bin/bash
# Cleanup script for Trabas E2E tests
# This script stops all services and cleans up test artifacts

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[E2E Cleanup]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[E2E Cleanup]${NC} $1"
}

error() {
    echo -e "${RED}[E2E Cleanup]${NC} $1"
}

log "Starting cleanup of Trabas E2E test environment..."

# Stop Trabas client
if [ -f /tmp/trabas_client.pid ]; then
    CLIENT_PID=$(cat /tmp/trabas_client.pid)
    if ps -p $CLIENT_PID > /dev/null 2>&1; then
        log "Stopping Trabas client (PID: $CLIENT_PID)..."
        kill $CLIENT_PID 2>/dev/null || true
        sleep 1
        # Force kill if still running
        if ps -p $CLIENT_PID > /dev/null 2>&1; then
            warn "Force killing Trabas client..."
            kill -9 $CLIENT_PID 2>/dev/null || true
        fi
    fi
    rm -f /tmp/trabas_client.pid
fi

# Stop Trabas server
if [ -f /tmp/trabas_server.pid ]; then
    SERVER_PID=$(cat /tmp/trabas_server.pid)
    if ps -p $SERVER_PID > /dev/null 2>&1; then
        log "Stopping Trabas server (PID: $SERVER_PID)..."
        kill $SERVER_PID 2>/dev/null || true
        sleep 1
        # Force kill if still running
        if ps -p $SERVER_PID > /dev/null 2>&1; then
            warn "Force killing Trabas server..."
            kill -9 $SERVER_PID 2>/dev/null || true
        fi
    fi
    rm -f /tmp/trabas_server.pid
fi

# Stop mock server
if [ -f /tmp/mock_server.pid ]; then
    MOCK_PID=$(cat /tmp/mock_server.pid)
    if ps -p $MOCK_PID > /dev/null 2>&1; then
        log "Stopping mock server (PID: $MOCK_PID)..."
        kill $MOCK_PID 2>/dev/null || true
        sleep 1
        # Force kill if still running
        if ps -p $MOCK_PID > /dev/null 2>&1; then
            warn "Force killing mock server..."
            kill -9 $MOCK_PID 2>/dev/null || true
        fi
    fi
    rm -f /tmp/mock_server.pid
fi

# Kill any remaining trabas processes
log "Checking for any remaining trabas processes..."
REMAINING_TRABAS=$(pgrep -f trabas || true)
if [ -n "$REMAINING_TRABAS" ]; then
    warn "Found remaining trabas processes: $REMAINING_TRABAS"
    pkill -f trabas 2>/dev/null || true
    sleep 1
    # Force kill if still running
    pkill -9 -f trabas 2>/dev/null || true
fi

# Kill any remaining mock server processes
REMAINING_MOCK=$(pgrep -f mock_server.py || true)
if [ -n "$REMAINING_MOCK" ]; then
    warn "Found remaining mock server processes: $REMAINING_MOCK"
    pkill -f mock_server.py 2>/dev/null || true
    sleep 1
    # Force kill if still running
    pkill -9 -f mock_server.py 2>/dev/null || true
fi

# Clean up test workspace (optional, uncomment if needed)
# log "Cleaning up test workspace..."
# rm -rf /tmp/trabas_test

# Show final process status
log "Final process check:"
TRABAS_PROCS=$(pgrep -f trabas || true)
MOCK_PROCS=$(pgrep -f mock_server.py || true)

if [ -z "$TRABAS_PROCS" ] && [ -z "$MOCK_PROCS" ]; then
    log "All processes cleaned up successfully"
else
    if [ -n "$TRABAS_PROCS" ]; then
        error "Some trabas processes still running: $TRABAS_PROCS"
    fi
    if [ -n "$MOCK_PROCS" ]; then
        error "Some mock server processes still running: $MOCK_PROCS"
    fi
fi

log "Cleanup complete!"
