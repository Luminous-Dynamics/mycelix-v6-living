---
sidebar_position: 2
title: REST API Reference
---

# REST API Reference

HTTP REST API for Mycelix server operations.

## Base URL

```
http://localhost:8080/api/v1
https://mycelix.example.com/api/v1
```

## Authentication

Include credentials in request headers:

```bash
# API Key
curl -H "X-API-Key: mk_prod_xxx" https://mycelix.example.com/api/v1/cycle

# JWT Bearer Token
curl -H "Authorization: Bearer eyJhbG..." https://mycelix.example.com/api/v1/cycle
```

## Response Format

All responses follow this structure:

```json
{
  "success": true,
  "data": { },
  "meta": {
    "phase": "Surge",
    "cycleDay": 10,
    "timestamp": "2024-03-15T14:30:00.000Z"
  }
}
```

Error responses:

```json
{
  "success": false,
  "error": {
    "code": "NOT_FOUND",
    "message": "Primitive not found",
    "details": { }
  },
  "meta": {
    "phase": "Surge",
    "timestamp": "2024-03-15T14:30:00.000Z"
  }
}
```

## Cycle Endpoints

### GET /cycle

Get current cycle status.

**Response**

```json
{
  "success": true,
  "data": {
    "phase": "Surge",
    "day": 10,
    "cycleNumber": 5,
    "progress": 0.357,
    "nextPhase": "Settle",
    "daysUntilNextPhase": 4,
    "startDate": "2024-01-01",
    "currentCycleStart": "2024-03-08"
  }
}
```

### GET /cycle/history

Get cycle history.

**Query Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| limit | number | Number of cycles to return (default: 10) |
| offset | number | Offset for pagination |

**Response**

```json
{
  "success": true,
  "data": {
    "cycles": [
      {
        "number": 5,
        "startDate": "2024-03-08",
        "endDate": null,
        "status": "active"
      },
      {
        "number": 4,
        "startDate": "2024-02-09",
        "endDate": "2024-03-07",
        "status": "completed",
        "summary": {
          "totalEvents": 125000,
          "peakThroughput": 5000
        }
      }
    ],
    "total": 5
  }
}
```

## Primitive Endpoints

### GET /primitives

List all primitives.

**Query Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| type | string | Filter by primitive type |
| status | string | Filter by status (active, paused, error) |
| limit | number | Results per page (default: 50) |
| offset | number | Pagination offset |

**Response**

```json
{
  "success": true,
  "data": {
    "primitives": [
      {
        "id": "pulse-abc123",
        "type": "pulse",
        "name": "heartbeat",
        "status": "active",
        "createdAt": "2024-03-01T00:00:00.000Z",
        "updatedAt": "2024-03-15T14:00:00.000Z"
      }
    ],
    "total": 15,
    "limit": 50,
    "offset": 0
  }
}
```

### GET /primitives/:id

Get primitive by ID.

**Response**

```json
{
  "success": true,
  "data": {
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
      "errorCount": 3,
      "lastEmit": "2024-03-15T14:29:55.000Z",
      "avgDuration": 12
    },
    "createdAt": "2024-03-01T00:00:00.000Z",
    "updatedAt": "2024-03-15T14:00:00.000Z"
  }
}
```

### POST /primitives

Create a new primitive.

**Request Body**

```json
{
  "type": "pulse",
  "name": "metrics-collector",
  "config": {
    "interval": "30s",
    "emit": {
      "type": "metrics",
      "include": ["cpu", "memory"]
    }
  }
}
```

**Response**

```json
{
  "success": true,
  "data": {
    "id": "pulse-def456",
    "type": "pulse",
    "name": "metrics-collector",
    "status": "active",
    "createdAt": "2024-03-15T14:30:00.000Z"
  }
}
```

### PUT /primitives/:id

Update a primitive.

**Request Body**

```json
{
  "config": {
    "interval": "1m"
  }
}
```

**Response**

```json
{
  "success": true,
  "data": {
    "id": "pulse-abc123",
    "updated": true,
    "version": 3
  }
}
```

### DELETE /primitives/:id

Delete a primitive.

**Response**

```json
{
  "success": true,
  "data": {
    "id": "pulse-abc123",
    "deleted": true
  }
}
```

### POST /primitives/:id/invoke

Invoke a primitive.

**Request Body**

```json
{
  "payload": {
    "action": "process",
    "data": { "key": "value" }
  },
  "timeout": "30s"
}
```

**Response**

```json
{
  "success": true,
  "data": {
    "result": { "processed": true },
    "duration": 45,
    "phase": "Surge"
  }
}
```

### POST /primitives/:id/pause

Pause a primitive.

**Response**

```json
{
  "success": true,
  "data": {
    "id": "pulse-abc123",
    "status": "paused"
  }
}
```

### POST /primitives/:id/resume

Resume a paused primitive.

**Response**

```json
{
  "success": true,
  "data": {
    "id": "pulse-abc123",
    "status": "active"
  }
}
```

## Storage Endpoints

### GET /store/:key

Get a value from storage.

**Query Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| store | string | Storage type: root, mycelium, archive |

**Response**

```json
{
  "success": true,
  "data": {
    "key": "user:123",
    "value": { "name": "Alice", "role": "admin" },
    "metadata": {
      "version": 5,
      "createdAt": "2024-03-01T00:00:00.000Z",
      "updatedAt": "2024-03-15T10:00:00.000Z",
      "ttl": null
    }
  }
}
```

### PUT /store/:key

Set a value in storage.

**Query Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| store | string | Storage type: root, mycelium, archive |

**Request Body**

```json
{
  "value": { "name": "Alice", "role": "admin" },
  "ttl": "24h"
}
```

**Response**

```json
{
  "success": true,
  "data": {
    "key": "user:123",
    "version": 6,
    "stored": true
  }
}
```

### DELETE /store/:key

Delete a value from storage.

**Response**

```json
{
  "success": true,
  "data": {
    "key": "user:123",
    "deleted": true
  }
}
```

### GET /store

List keys in storage.

**Query Parameters**

| Parameter | Type | Description |
|-----------|------|-------------|
| store | string | Storage type |
| prefix | string | Key prefix filter |
| limit | number | Max results |
| cursor | string | Pagination cursor |

**Response**

```json
{
  "success": true,
  "data": {
    "keys": ["user:123", "user:456", "user:789"],
    "nextCursor": "abc123",
    "hasMore": true
  }
}
```

## Cluster Endpoints

### GET /cluster

Get cluster status.

**Response**

```json
{
  "success": true,
  "data": {
    "clusterId": "mycelix-prod",
    "leader": "node-1",
    "nodes": [
      {
        "id": "node-1",
        "address": "10.0.0.1:9090",
        "status": "healthy",
        "role": "leader",
        "phase": "Surge",
        "metrics": {
          "cpu": 0.45,
          "memory": 0.62,
          "connections": 150
        }
      },
      {
        "id": "node-2",
        "address": "10.0.0.2:9090",
        "status": "healthy",
        "role": "follower",
        "phase": "Surge",
        "metrics": {
          "cpu": 0.52,
          "memory": 0.58,
          "connections": 145
        }
      }
    ],
    "healthy": 2,
    "total": 2
  }
}
```

### GET /cluster/nodes/:id

Get specific node details.

**Response**

```json
{
  "success": true,
  "data": {
    "id": "node-1",
    "address": "10.0.0.1:9090",
    "status": "healthy",
    "role": "leader",
    "uptime": 864000,
    "version": "1.0.0",
    "primitives": {
      "active": 15,
      "paused": 2,
      "error": 0
    },
    "metrics": {
      "cpu": 0.45,
      "memory": 0.62,
      "disk": 0.30,
      "network": {
        "rxBytes": 1234567890,
        "txBytes": 987654321
      }
    }
  }
}
```

## Health Endpoints

### GET /health

Basic health check.

**Response**

```json
{
  "status": "healthy",
  "timestamp": "2024-03-15T14:30:00.000Z"
}
```

### GET /health/ready

Readiness check.

**Response**

```json
{
  "ready": true,
  "checks": {
    "storage": "ok",
    "cluster": "ok",
    "primitives": "ok"
  }
}
```

### GET /health/live

Liveness check.

**Response**

```json
{
  "alive": true,
  "uptime": 864000
}
```

## Metrics Endpoint

### GET /metrics

Prometheus-format metrics.

**Response**

```
# HELP mycelix_cycle_day Current day in the cycle
# TYPE mycelix_cycle_day gauge
mycelix_cycle_day 10

# HELP mycelix_primitives_active Number of active primitives
# TYPE mycelix_primitives_active gauge
mycelix_primitives_active{type="pulse"} 5
mycelix_primitives_active{type="thread"} 10

# HELP mycelix_requests_total Total HTTP requests
# TYPE mycelix_requests_total counter
mycelix_requests_total{method="GET",path="/api/v1/cycle"} 12345
```

## Error Codes

| HTTP Status | Code | Description |
|-------------|------|-------------|
| 400 | BAD_REQUEST | Invalid request parameters |
| 401 | UNAUTHORIZED | Authentication required |
| 403 | FORBIDDEN | Insufficient permissions |
| 404 | NOT_FOUND | Resource not found |
| 409 | CONFLICT | Resource conflict |
| 422 | VALIDATION_ERROR | Validation failed |
| 429 | RATE_LIMITED | Too many requests |
| 500 | INTERNAL_ERROR | Server error |
| 503 | UNAVAILABLE | Service unavailable |

## Rate Limits

Rate limits vary by phase:

| Phase | Requests/min | Burst |
|-------|--------------|-------|
| Dawn | 500 | 50 |
| Surge | 1000 | 100 |
| Settle | 700 | 70 |
| Rest | 200 | 20 |

Rate limit headers:

```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 950
X-RateLimit-Reset: 1710513600
X-Mycelix-Phase: Surge
```

## OpenAPI Specification

Full OpenAPI specification available at:

```
GET /openapi.yaml
GET /openapi.json
```

Interactive documentation:

```
GET /docs (Swagger UI)
GET /redoc (ReDoc)
```

## Next Steps

- [GraphQL Schema](./graphql) - GraphQL API documentation
- [WebSocket RPC](./websocket) - Real-time API documentation
