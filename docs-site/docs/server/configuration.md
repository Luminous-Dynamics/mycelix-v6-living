---
sidebar_position: 1
title: Configuration Reference
---

# Server Configuration Reference

Complete reference for configuring the Mycelix server.

## Configuration File

Mycelix looks for configuration in this order:

1. `mycelix.config.ts` (TypeScript)
2. `mycelix.config.js` (JavaScript)
3. `mycelix.config.json` (JSON)
4. Environment variables

### TypeScript Configuration

```typescript
// mycelix.config.ts
import { defineConfig } from '@mycelix/core';

export default defineConfig({
  // All options documented below
});
```

## Core Configuration

### Cycle Settings

```typescript
export default defineConfig({
  cycle: {
    // When the first cycle began (ISO 8601 date)
    startDate: '2024-01-01',

    // Timezone for day calculations
    timezone: 'UTC',

    // Time of day for phase transitions (24h format)
    transitionTime: '00:00',

    // Duration of transition window (gradual change)
    transitionWindow: '1h',

    // Custom phase lengths (must sum to 28)
    phases: {
      Dawn: 7,
      Surge: 7,
      Settle: 7,
      Rest: 7,
    },
  },
});
```

### Server Settings

```typescript
export default defineConfig({
  server: {
    // HTTP/REST server
    http: {
      enabled: true,
      host: '0.0.0.0',
      port: 8080,
      cors: {
        origin: '*',
        methods: ['GET', 'POST', 'PUT', 'DELETE'],
      },
    },

    // WebSocket server
    ws: {
      enabled: true,
      host: '0.0.0.0',
      port: 9090,
      path: '/ws',
      pingInterval: 30000,
      maxPayloadSize: '1mb',
    },

    // GraphQL server
    graphql: {
      enabled: true,
      path: '/graphql',
      playground: true,
      introspection: true,
    },
  },
});
```

### Primitive Configuration

```typescript
export default defineConfig({
  primitives: {
    // Enable specific primitives
    enabled: [
      'pulse', 'signal', 'echo',
      'thread', 'weave', 'mesh',
      'spore', 'bloom', 'fruit',
      'root', 'mycelium', 'archive',
      'stream', 'pool', 'gate',
      'sense', 'dream', 'wake',
      'cycle', 'phase', 'rhythm',
    ],

    // Global primitive defaults
    defaults: {
      timeout: '30s',
      retries: 3,
      backoff: 'exponential',
    },

    // Per-primitive configuration
    pulse: {
      defaultInterval: '1m',
      maxConcurrent: 100,
    },

    thread: {
      maxConcurrency: 10,
      queueSize: 1000,
    },

    pool: {
      defaultSize: 10,
      maxSize: 100,
    },
  },
});
```

## Storage Configuration

### Local Storage (Root)

```typescript
export default defineConfig({
  storage: {
    root: {
      driver: 'memory', // 'memory' | 'file' | 'redis'
      path: './data/root',
      maxSize: '1gb',
      ttl: '24h',
    },
  },
});
```

### Distributed Storage (Mycelium)

```typescript
export default defineConfig({
  storage: {
    mycelium: {
      driver: 'raft', // 'raft' | 'gossip' | 'external'
      replication: 3,
      consistency: 'eventual', // 'eventual' | 'strong'
      external: {
        // When driver is 'external'
        url: 'redis://localhost:6379',
      },
    },
  },
});
```

### Archive Storage

```typescript
export default defineConfig({
  storage: {
    archive: {
      driver: 'file', // 'file' | 's3' | 'gcs'
      path: './data/archive',
      compression: 'zstd',
      retention: '365d',

      // S3 configuration
      s3: {
        bucket: 'mycelix-archive',
        region: 'us-east-1',
        prefix: 'archive/',
      },
    },
  },
});
```

## Networking Configuration

### Cluster Settings

```typescript
export default defineConfig({
  cluster: {
    // Node identity
    nodeId: 'auto', // 'auto' generates UUID

    // Discovery method
    discovery: {
      method: 'multicast', // 'multicast' | 'dns' | 'static' | 'kubernetes'

      // Static node list
      nodes: [
        'mycelix-1:9090',
        'mycelix-2:9090',
        'mycelix-3:9090',
      ],

      // DNS-based discovery
      dns: {
        service: '_mycelix._tcp.cluster.local',
      },

      // Kubernetes discovery
      kubernetes: {
        namespace: 'default',
        labelSelector: 'app=mycelix',
      },
    },

    // Gossip protocol settings
    gossip: {
      port: 7946,
      fanout: 3,
      interval: '1s',
    },
  },
});
```

### TLS Configuration

```typescript
export default defineConfig({
  tls: {
    enabled: true,
    cert: '/path/to/cert.pem',
    key: '/path/to/key.pem',
    ca: '/path/to/ca.pem',

    // Mutual TLS
    clientAuth: true,
    clientCa: '/path/to/client-ca.pem',
  },
});
```

## Observability Configuration

### Logging

```typescript
export default defineConfig({
  logging: {
    level: 'info', // 'trace' | 'debug' | 'info' | 'warn' | 'error'
    format: 'json', // 'json' | 'pretty'
    output: 'stdout', // 'stdout' | 'file' | 'both'
    file: {
      path: './logs/mycelix.log',
      rotation: 'daily',
      retention: '30d',
    },
  },
});
```

### Metrics

```typescript
export default defineConfig({
  metrics: {
    enabled: true,
    port: 9091,
    path: '/metrics',
    format: 'prometheus', // 'prometheus' | 'statsd' | 'otlp'

    // Metric labels
    labels: {
      service: 'mycelix',
      environment: 'production',
    },

    // Phase-specific collection intervals
    interval: {
      Dawn: '30s',
      Surge: '10s',
      Settle: '30s',
      Rest: '60s',
    },
  },
});
```

### Tracing

```typescript
export default defineConfig({
  tracing: {
    enabled: true,
    exporter: 'otlp', // 'otlp' | 'jaeger' | 'zipkin'
    endpoint: 'http://localhost:4318',
    sampleRate: 0.1,

    // Phase-specific sampling
    phaseSampling: {
      Dawn: 0.5,
      Surge: 0.01,
      Settle: 0.1,
      Rest: 0.5,
    },
  },
});
```

## Phase-Specific Configuration

Override any setting per phase:

```typescript
export default defineConfig({
  // Base configuration
  server: {
    http: { port: 8080 },
  },

  // Phase overrides
  phaseOverrides: {
    Dawn: {
      primitives: {
        thread: { maxConcurrency: 5 },
      },
    },
    Surge: {
      primitives: {
        thread: { maxConcurrency: 20 },
        pool: { defaultSize: 50 },
      },
      metrics: {
        interval: '5s',
      },
    },
    Rest: {
      primitives: {
        thread: { maxConcurrency: 2 },
      },
      server: {
        ws: { maxPayloadSize: '100kb' },
      },
    },
  },
});
```

## Environment Variables

All configuration can be overridden via environment variables:

```bash
# Server
MYCELIX_HTTP_PORT=8080
MYCELIX_WS_PORT=9090
MYCELIX_LOG_LEVEL=debug

# Cycle
MYCELIX_CYCLE_START=2024-01-01
MYCELIX_CYCLE_TIMEZONE=America/New_York

# Storage
MYCELIX_STORAGE_DRIVER=redis
MYCELIX_REDIS_URL=redis://localhost:6379

# Cluster
MYCELIX_NODE_ID=node-1
MYCELIX_CLUSTER_NODES=node-1:9090,node-2:9090

# TLS
MYCELIX_TLS_ENABLED=true
MYCELIX_TLS_CERT=/etc/mycelix/cert.pem
MYCELIX_TLS_KEY=/etc/mycelix/key.pem
```

## Configuration Validation

Validate configuration before starting:

```bash
mycelix config validate
mycelix config show
mycelix config diff --env production
```

## Complete Example

```typescript
// mycelix.config.ts
import { defineConfig } from '@mycelix/core';

export default defineConfig({
  cycle: {
    startDate: '2024-01-01',
    timezone: 'UTC',
  },

  server: {
    http: { port: 8080 },
    ws: { port: 9090 },
    graphql: { enabled: true },
  },

  primitives: {
    enabled: ['pulse', 'thread', 'stream', 'gate'],
    thread: { maxConcurrency: 10 },
  },

  storage: {
    root: { driver: 'memory' },
    mycelium: { driver: 'raft', replication: 3 },
    archive: { driver: 's3', compression: 'zstd' },
  },

  cluster: {
    discovery: { method: 'kubernetes' },
  },

  logging: { level: 'info', format: 'json' },
  metrics: { enabled: true, format: 'prometheus' },
  tracing: { enabled: true, exporter: 'otlp' },

  phaseOverrides: {
    Surge: {
      primitives: { thread: { maxConcurrency: 50 } },
    },
  },
});
```

## Next Steps

- [Deployment Guide](./deployment) - Deploy to production
- [Security Configuration](./security) - Secure your deployment
