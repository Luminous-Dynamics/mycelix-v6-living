# ws-server

WebSocket RPC server for the Mycelix Living Protocol.

## Overview

This crate provides a production-ready WebSocket server that exposes the Living Protocol through a JSON-RPC style API:

- **WebSocket RPC**: Real-time bidirectional communication
- **HTTP Health/Metrics**: Monitoring endpoints
- **Authentication**: Optional JWT support
- **Persistence**: SQLite or PostgreSQL backends
- **Observability**: OpenTelemetry integration

## Installation

```toml
[dependencies]
ws-server = "0.6"
```

Or install the binary:

```bash
cargo install ws-server
```

## Running the Server

```bash
# Start with defaults (localhost:8888)
mycelix-ws-server

# Custom host and port
mycelix-ws-server --host 0.0.0.0 --port 9000

# With debug logging
mycelix-ws-server --log-level debug

# With simulated time (for testing)
mycelix-ws-server --simulated-time --time-acceleration 100
```

## Docker

```bash
docker run -p 8888:8888 -p 8889:8889 ghcr.io/mycelix/mycelix-v6-living:latest
```

## API Reference

### WebSocket RPC (port 8888)

**Request Format:**
```json
{ "id": "1", "method": "getCycleState", "params": {} }
```

**Available Methods:**

| Method | Description |
|--------|-------------|
| `getCycleState` | Get full cycle state |
| `getCurrentPhase` | Get current phase name |
| `getCycleNumber` | Get current cycle number |
| `getTransitionHistory` | Get phase transition history |
| `getPhaseMetrics` | Get metrics for a phase |
| `isOperationPermitted` | Check if operation is allowed |

### HTTP Endpoints (port 8889)

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check |
| `GET /metrics` | Server metrics |
| `GET /state` | Current cycle state |

## Feature Flags

```toml
[features]
default = ["sqlite"]
jwt = []          # JWT authentication
graphql = []      # GraphQL API
sse = []          # Server-Sent Events
webhooks = []     # Webhook notifications
sqlite = []       # SQLite persistence
postgres = []     # PostgreSQL persistence
otlp = []         # OpenTelemetry export
```

## License

AGPL-3.0-or-later
