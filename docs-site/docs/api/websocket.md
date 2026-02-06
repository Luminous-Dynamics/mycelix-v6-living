---
sidebar_position: 1
title: WebSocket RPC Reference
---

# WebSocket RPC Reference

Real-time communication with Mycelix using WebSocket JSON-RPC.

## Connection

### Endpoint

```
ws://localhost:9090/ws
wss://mycelix.example.com/ws  # TLS
```

### Authentication

Include credentials in the connection request:

```typescript
// Using API Key
const ws = new WebSocket('wss://mycelix.example.com/ws', {
  headers: {
    'X-API-Key': 'mk_prod_xxx',
  },
});

// Using JWT
const ws = new WebSocket('wss://mycelix.example.com/ws', {
  headers: {
    'Authorization': 'Bearer eyJhbG...',
  },
});
```

## Message Format

All messages use JSON-RPC 2.0:

### Request

```json
{
  "jsonrpc": "2.0",
  "id": "unique-request-id",
  "method": "method.name",
  "params": { }
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "id": "unique-request-id",
  "result": { }
}
```

### Error Response

```json
{
  "jsonrpc": "2.0",
  "id": "unique-request-id",
  "error": {
    "code": -32600,
    "message": "Invalid request",
    "data": { }
  }
}
```

### Notification (Server-to-Client)

```json
{
  "jsonrpc": "2.0",
  "method": "event.name",
  "params": { }
}
```

## Core Methods

### cycle.status

Get current cycle state.

```json
// Request
{
  "jsonrpc": "2.0",
  "id": "1",
  "method": "cycle.status"
}

// Response
{
  "jsonrpc": "2.0",
  "id": "1",
  "result": {
    "phase": "Dawn",
    "day": 3,
    "cycleNumber": 5,
    "progress": 0.107,
    "nextPhase": "Surge",
    "daysUntilNextPhase": 4,
    "timestamp": "2024-03-15T14:30:00.000Z"
  }
}
```

### cycle.subscribe

Subscribe to cycle events.

```json
// Request
{
  "jsonrpc": "2.0",
  "id": "2",
  "method": "cycle.subscribe",
  "params": {
    "events": ["phaseChange", "dayChange", "cycleComplete"]
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "2",
  "result": {
    "subscriptionId": "sub-abc123"
  }
}

// Notification (when phase changes)
{
  "jsonrpc": "2.0",
  "method": "cycle.phaseChange",
  "params": {
    "subscriptionId": "sub-abc123",
    "from": "Dawn",
    "to": "Surge",
    "day": 8,
    "cycleNumber": 5,
    "timestamp": "2024-03-22T00:00:00.000Z"
  }
}
```

## Primitive Methods

### primitive.list

List all registered primitives.

```json
// Request
{
  "jsonrpc": "2.0",
  "id": "3",
  "method": "primitive.list",
  "params": {
    "type": "pulse",  // optional filter
    "status": "active"  // optional filter
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "3",
  "result": {
    "primitives": [
      {
        "id": "pulse-abc123",
        "type": "pulse",
        "name": "heartbeat",
        "status": "active",
        "createdAt": "2024-03-01T00:00:00.000Z"
      }
    ],
    "total": 1
  }
}
```

### primitive.get

Get primitive details.

```json
// Request
{
  "jsonrpc": "2.0",
  "id": "4",
  "method": "primitive.get",
  "params": {
    "id": "pulse-abc123"
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "4",
  "result": {
    "id": "pulse-abc123",
    "type": "pulse",
    "name": "heartbeat",
    "status": "active",
    "config": {
      "interval": {
        "Dawn": "10s",
        "Surge": "1s",
        "Settle": "5s",
        "Rest": "30s"
      }
    },
    "stats": {
      "emitCount": 12450,
      "lastEmit": "2024-03-15T14:29:55.000Z"
    }
  }
}
```

### primitive.create

Create a new primitive.

```json
// Request
{
  "jsonrpc": "2.0",
  "id": "5",
  "method": "primitive.create",
  "params": {
    "type": "pulse",
    "name": "metrics",
    "config": {
      "interval": "30s",
      "emit": {
        "type": "metrics",
        "include": ["cpu", "memory", "connections"]
      }
    }
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "5",
  "result": {
    "id": "pulse-def456",
    "created": true
  }
}
```

### primitive.invoke

Invoke a primitive directly.

```json
// Request
{
  "jsonrpc": "2.0",
  "id": "6",
  "method": "primitive.invoke",
  "params": {
    "id": "thread-abc123",
    "payload": {
      "action": "process",
      "data": { "key": "value" }
    }
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "6",
  "result": {
    "response": { "processed": true },
    "duration": 45,
    "phase": "Surge"
  }
}
```

### primitive.subscribe

Subscribe to primitive events.

```json
// Request
{
  "jsonrpc": "2.0",
  "id": "7",
  "method": "primitive.subscribe",
  "params": {
    "id": "pulse-abc123",
    "events": ["emit", "error", "stateChange"]
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "7",
  "result": {
    "subscriptionId": "sub-xyz789"
  }
}

// Notification (on pulse emit)
{
  "jsonrpc": "2.0",
  "method": "primitive.emit",
  "params": {
    "subscriptionId": "sub-xyz789",
    "primitiveId": "pulse-abc123",
    "data": {
      "status": "alive",
      "uptime": 86400
    },
    "timestamp": "2024-03-15T14:30:00.000Z"
  }
}
```

## Storage Methods

### store.get

Get a value from storage.

```json
// Request
{
  "jsonrpc": "2.0",
  "id": "8",
  "method": "store.get",
  "params": {
    "key": "user:123",
    "store": "root"  // 'root' | 'mycelium' | 'archive'
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "8",
  "result": {
    "value": { "name": "Alice", "role": "admin" },
    "metadata": {
      "createdAt": "2024-03-01T00:00:00.000Z",
      "updatedAt": "2024-03-15T10:00:00.000Z",
      "version": 5
    }
  }
}
```

### store.set

Set a value in storage.

```json
// Request
{
  "jsonrpc": "2.0",
  "id": "9",
  "method": "store.set",
  "params": {
    "key": "user:123",
    "value": { "name": "Alice", "role": "admin" },
    "store": "root",
    "ttl": "24h"
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "9",
  "result": {
    "success": true,
    "version": 6
  }
}
```

### store.delete

Delete a value from storage.

```json
// Request
{
  "jsonrpc": "2.0",
  "id": "10",
  "method": "store.delete",
  "params": {
    "key": "user:123",
    "store": "root"
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "10",
  "result": {
    "deleted": true
  }
}
```

## Cluster Methods

### cluster.status

Get cluster status.

```json
// Request
{
  "jsonrpc": "2.0",
  "id": "11",
  "method": "cluster.status"
}

// Response
{
  "jsonrpc": "2.0",
  "id": "11",
  "result": {
    "nodes": [
      {
        "id": "node-1",
        "address": "10.0.0.1:9090",
        "status": "healthy",
        "phase": "Surge",
        "load": 0.45
      },
      {
        "id": "node-2",
        "address": "10.0.0.2:9090",
        "status": "healthy",
        "phase": "Surge",
        "load": 0.52
      }
    ],
    "leader": "node-1",
    "healthy": 2,
    "total": 2
  }
}
```

## Batch Requests

Send multiple requests in a single message:

```json
// Request
[
  { "jsonrpc": "2.0", "id": "1", "method": "cycle.status" },
  { "jsonrpc": "2.0", "id": "2", "method": "cluster.status" },
  { "jsonrpc": "2.0", "id": "3", "method": "primitive.list" }
]

// Response
[
  { "jsonrpc": "2.0", "id": "1", "result": { "phase": "Surge", ... } },
  { "jsonrpc": "2.0", "id": "2", "result": { "nodes": [...], ... } },
  { "jsonrpc": "2.0", "id": "3", "result": { "primitives": [...], ... } }
]
```

## Error Codes

| Code | Message | Description |
|------|---------|-------------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid request | Not a valid JSON-RPC request |
| -32601 | Method not found | Method does not exist |
| -32602 | Invalid params | Invalid method parameters |
| -32603 | Internal error | Internal JSON-RPC error |
| -32000 | Server error | Generic server error |
| -32001 | Unauthorized | Authentication required |
| -32002 | Forbidden | Insufficient permissions |
| -32003 | Not found | Resource not found |
| -32004 | Rate limited | Too many requests |
| -32005 | Phase restricted | Operation not allowed in current phase |

## Client Examples

### JavaScript/TypeScript

```typescript
import { MycelixClient } from '@mycelix/sdk';

const client = new MycelixClient('wss://mycelix.example.com/ws', {
  apiKey: 'mk_prod_xxx',
});

// Subscribe to cycle changes
client.subscribe('cycle', ['phaseChange'], (event) => {
  console.log(`Phase changed to ${event.to}`);
});

// Invoke a primitive
const result = await client.invoke('thread-abc123', {
  action: 'process',
  data: { key: 'value' },
});
```

### Python

```python
from mycelix import MycelixClient

client = MycelixClient('wss://mycelix.example.com/ws', api_key='mk_prod_xxx')

# Get cycle status
status = await client.cycle.status()
print(f"Current phase: {status.phase}")

# Subscribe to events
@client.on('cycle.phaseChange')
async def on_phase_change(event):
    print(f"Phase changed to {event['to']}")
```

## Next Steps

- [REST API Reference](./rest) - HTTP API documentation
- [GraphQL Schema](./graphql) - GraphQL API documentation
