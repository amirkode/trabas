# Trabas End-to-End Tests

This directory contains comprehensive end-to-end tests for the Trabas HTTP tunneling system. These tests verify the complete data flow from public requests through the Trabas server and client to the underlying service.

## Files

- **`mock_server.py`** - Mock HTTP server that simulates underlying services
- **`run_tests.py`** - Comprehensive E2E test suite
- **`setup.sh`** - Automated setup script for the test environment
- **`cleanup.sh`** - Cleanup script to stop all services and clean up
- **`README.md`** - This documentation

## Quick Start

### Prerequisites

- Python 3.x with `requests` library
- Built Trabas binary (`cargo build --release --manifest-path cli/Cargo.toml`)

### Running Tests

#### Option 1: Full Automated Run
```bash
# Setup, run tests, and cleanup
./tests/e2e/setup.sh && \
python3 tests/e2e/run_tests.py && \
./tests/e2e/cleanup.sh
```

#### Option 2: Manual Control
```bash
# 1. Setup the environment
./tests/e2e/setup.sh

# 2. Run tests
python3 tests/e2e/run_tests.py

# 3. Cleanup when done
./tests/e2e/cleanup.sh
```

## Test Coverage

### Basic Functionality
- [x] **Ping/Pong** - Basic connectivity test
- [x] **Path-based routing** - `server:8001/client-id/path`
- [x] **Query parameter routing** - `server:8001/path?trabas_client_id=client-id`

### HTTP Methods
- [x] **GET requests** - Various response types
- [x] **POST requests** - JSON and text payloads
- [x] **PUT requests** - Update operations
- [x] **DELETE requests** - Delete operations

### Data Handling
- [x] **JSON responses** - Structured data
- [x] **Headers forwarding** - Custom headers preservation
- [x] **Large payloads** - 10KB+ data transmission
- [x] **Different status codes** - 200, 201, 400, 500, etc.

### Security
- [ ] **TLS Behind NGINX** - Ensure TLS termination is handled correctly
- [ ] **TLS Direct** - Test direct TLS connections to the Trabas server

### Performance & Reliability
- [x] **Slow requests** - 1+ second responses
- [x] **Timeout handling** - Request timeout management
- [x] **Error scenarios** - Non-existent client IDs

### Scalability Tests
- [ ] **Resource exhaustion (Stress Test)** - Simulate high load and resource limits
- [ ] **Multiple Server Instances** - Run multiple Trabas servers (Ofc with redis) and clients

## Mock Server Endpoints

The mock server provides the following test endpoints:

| Method | Path | Description |
|--------|------|-------------|
| GET | `/ping` | Simple ping/pong response |
| GET | `/json` | JSON response with timestamp |
| GET | `/slow` | Slow response (1 second delay) |
| GET | `/headers` | Echo request headers |
| GET | `/status/{code}` | Return specific HTTP status code |
| POST | `/echo` | Echo complete request details |
| POST | `/json-echo` | Echo JSON request body |
| PUT | `/*` | Echo PUT request details |
| DELETE | `/*` | Echo DELETE request details |

## Configuration

### Environment Variables

The setup script supports the following environment variables:

```bash
WORKSPACE_DIR="/tmp/trabas_test"    # Test workspace directory
TRABAS_BINARY="./target/release/trabas"  # Path to trabas binary
MOCK_SERVER_PORT="3000"            # Mock server port
PUBLIC_PORT="8001"                 # Trabas public port
CLIENT_PORT="8002"                 # Trabas client connection port
CLIENT_ID="e2e-test-client"        # Test client ID
```

### Test Parameters

The test runner accepts command-line arguments:

```bash
python3 tests/e2e/run_tests.py \
  --server-url http://localhost:8001 \
  --client-id e2e-test-client \
  --timeout 10
```

## CI/CD Integration

These tests are designed to run in GitHub Actions and other CI environments. See `.github/workflows/test.yml` for the integration example.

### GitHub Actions Usage

The tests run as part of the `end-to-end-tests` job:

1. **Build** - Compile the release binary
2. **Setup** - Configure Trabas and start services
3. **Test** - Run the comprehensive test suite
4. **Cleanup** - Stop services and clean up resources

## Architecture

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   E2E Test  │───▶│   Trabas    │───▶│   Trabas    │───▶│    Mock     │
│   Script    │    │   Server    │    │   Client    │    │   Server    │
│             │    │ :8001,:8002 │    │             │    │   :3000     │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
       │                   │                   │                   │
       │                   │                   │                   │
   HTTP Requests     Public & Client      TCP Tunnel         HTTP Server
   (Test Cases)       Endpoints         (Trabas Protocol)   (Mock Responses)
```

## Troubleshooting

### Common Issues

1. **Port conflicts** - Ensure ports 3000, 8001, 8002 are available
2. **Binary not found** - Run `cargo build --release --manifest-path cli/Cargo.toml`
3. **Permission denied** - Ensure scripts are executable: `chmod +x tests/e2e/*.sh`
4. **Processes not stopping** - Run cleanup script: `./tests/e2e/cleanup.sh`

### Debug Mode

To see detailed output during tests:

```bash
# Enable debug logging
RUST_LOG=debug ./tests/e2e/setup.sh

# Run tests with verbose output
python3 tests/e2e/run_tests.py --timeout 30
```

### Manual Testing

You can also test manually while services are running:

```bash
# After running setup.sh
curl http://localhost:8001/e2e-test-client/ping
curl http://localhost:8001/e2e-test-client/json
curl -X POST http://localhost:8001/e2e-test-client/echo -d '{"test": "data"}'
```

## Contributing

When adding new tests:

1. Add test endpoints to `mock_server.py` if needed
2. Add test methods to `run_tests.py`
3. Update this README with new test coverage
4. Test both locally and in CI environment
