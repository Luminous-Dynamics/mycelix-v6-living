---
sidebar_position: 1
title: TypeScript SDK
---

# TypeScript SDK

Full-featured TypeScript/JavaScript SDK for Mycelix.

## Installation

```bash
npm install @mycelix/sdk
# or
pnpm add @mycelix/sdk
# or
bun add @mycelix/sdk
```

## Quick Start

```typescript
import { MycelixClient } from '@mycelix/sdk';

// Create client
const client = new MycelixClient({
  url: 'wss://mycelix.example.com/ws',
  apiKey: 'mk_prod_xxx',
});

// Connect
await client.connect();

// Get cycle status
const cycle = await client.cycle.status();
console.log(`Phase: ${cycle.phase}, Day: ${cycle.day}`);

// Invoke a primitive
const result = await client.primitives.invoke('thread-abc123', {
  action: 'process',
  data: { key: 'value' },
});
```

## Client Configuration

```typescript
import { MycelixClient, type ClientConfig } from '@mycelix/sdk';

const config: ClientConfig = {
  // Connection
  url: 'wss://mycelix.example.com/ws',

  // Authentication (choose one)
  apiKey: 'mk_prod_xxx',
  // or
  jwt: 'eyJhbG...',
  // or
  auth: async () => {
    // Dynamic token refresh
    return { type: 'bearer', token: await getToken() };
  },

  // Connection options
  autoConnect: true,
  reconnect: true,
  reconnectDelay: 1000,
  maxReconnectAttempts: 10,

  // Timeouts
  connectTimeout: 10000,
  requestTimeout: 30000,

  // Logging
  logger: console,
  logLevel: 'info',
};

const client = new MycelixClient(config);
```

## Cycle API

### Get Status

```typescript
const status = await client.cycle.status();

console.log(status.phase);           // 'Dawn' | 'Surge' | 'Settle' | 'Rest'
console.log(status.day);             // 1-28
console.log(status.cycleNumber);     // Current cycle number
console.log(status.progress);        // 0.0 - 1.0
console.log(status.nextPhase);       // Next phase name
console.log(status.daysUntilNextPhase);
```

### Subscribe to Changes

```typescript
// Phase changes
client.cycle.onPhaseChange((event) => {
  console.log(`Phase: ${event.from} → ${event.to}`);
  console.log(`Day: ${event.day}, Cycle: ${event.cycleNumber}`);
});

// Day changes
client.cycle.onDayChange((event) => {
  console.log(`Day ${event.day} of ${event.phase}`);
});

// Cycle complete
client.cycle.onCycleComplete((event) => {
  console.log(`Cycle ${event.cycleNumber} complete`);
});
```

### Phase-Conditional Logic

```typescript
import { Phase } from '@mycelix/sdk';

// Check current phase
if (client.cycle.currentPhase === Phase.Surge) {
  // High-throughput operations
}

// Wait for specific phase
await client.cycle.waitForPhase(Phase.Dawn);

// Execute only in specific phases
await client.cycle.duringPhase(Phase.Settle, async () => {
  await runAnalytics();
});
```

## Primitives API

### List Primitives

```typescript
const { primitives, total } = await client.primitives.list({
  type: 'pulse',
  status: 'active',
  limit: 50,
});

for (const p of primitives) {
  console.log(`${p.name} (${p.type}): ${p.status}`);
}
```

### Get Primitive

```typescript
const pulse = await client.primitives.get('pulse-abc123');

console.log(pulse.config);
console.log(pulse.stats.invocationCount);
console.log(pulse.stats.avgDuration);
```

### Create Primitive

```typescript
import { PrimitiveType } from '@mycelix/sdk';

const primitive = await client.primitives.create({
  type: PrimitiveType.Pulse,
  name: 'metrics-collector',
  config: {
    interval: {
      Dawn: '10s',
      Surge: '1s',
      Settle: '5s',
      Rest: '30s',
    },
    emit: {
      type: 'metrics',
      include: ['cpu', 'memory'],
    },
  },
});

console.log(`Created: ${primitive.id}`);
```

### Invoke Primitive

```typescript
// Simple invocation
const result = await client.primitives.invoke('thread-abc123', {
  action: 'process',
  data: { key: 'value' },
});

// With options
const result = await client.primitives.invoke('thread-abc123', payload, {
  timeout: 5000,
  priority: 'high',
});
```

### Subscribe to Primitive Events

```typescript
// Subscribe to emissions
const unsubscribe = await client.primitives.subscribe(
  'pulse-abc123',
  {
    onEmit: (data) => {
      console.log('Pulse emitted:', data);
    },
    onError: (error) => {
      console.error('Error:', error);
    },
    onStateChange: (from, to) => {
      console.log(`State: ${from} → ${to}`);
    },
  }
);

// Later: unsubscribe
unsubscribe();
```

### Pause/Resume

```typescript
await client.primitives.pause('pulse-abc123');
await client.primitives.resume('pulse-abc123');
```

### Delete

```typescript
await client.primitives.delete('pulse-abc123');
```

## Storage API

### Get/Set Values

```typescript
import { StoreType } from '@mycelix/sdk';

// Set value
await client.store.set('user:123', {
  name: 'Alice',
  role: 'admin',
}, {
  store: StoreType.Root,
  ttl: '24h',
});

// Get value
const user = await client.store.get<User>('user:123', {
  store: StoreType.Root,
});

// Delete value
await client.store.delete('user:123');
```

### List Keys

```typescript
const { keys, hasMore, cursor } = await client.store.keys({
  prefix: 'user:',
  store: StoreType.Root,
  limit: 100,
});
```

### Transactions

```typescript
await client.store.transaction(async (tx) => {
  const balance = await tx.get<number>('account:123:balance');
  await tx.set('account:123:balance', balance - 100);
  await tx.set('account:456:balance', balance + 100);
});
```

## Cluster API

### Get Status

```typescript
const cluster = await client.cluster.status();

console.log(`Nodes: ${cluster.healthy}/${cluster.total}`);
console.log(`Leader: ${cluster.leader?.id}`);

for (const node of cluster.nodes) {
  console.log(`${node.id}: ${node.status} (${node.role})`);
}
```

### Get Node Details

```typescript
const node = await client.cluster.node('node-1');

console.log(`CPU: ${node.metrics.cpu}`);
console.log(`Memory: ${node.metrics.memory}`);
console.log(`Connections: ${node.metrics.connections}`);
```

### Subscribe to Cluster Events

```typescript
client.cluster.onNodeStatusChange((event) => {
  console.log(`Node ${event.nodeId}: ${event.from} → ${event.to}`);
});
```

## Type Definitions

### Core Types

```typescript
import type {
  Phase,
  Cycle,
  Primitive,
  PrimitiveType,
  PrimitiveStatus,
  PrimitiveConfig,
  Node,
  Cluster,
  StoreEntry,
} from '@mycelix/sdk';
```

### Custom Primitive Types

```typescript
interface MyPrimitiveConfig {
  interval: Record<Phase, string>;
  emit: {
    type: string;
    include: string[];
  };
}

const pulse = await client.primitives.get<MyPrimitiveConfig>('pulse-abc123');
// pulse.config is typed as MyPrimitiveConfig
```

### Event Types

```typescript
import type {
  PhaseChangeEvent,
  DayChangeEvent,
  EmitEvent,
  StateChangeEvent,
  NodeStatusEvent,
} from '@mycelix/sdk';
```

## Error Handling

```typescript
import { MycelixError, ErrorCode } from '@mycelix/sdk';

try {
  await client.primitives.invoke('thread-abc123', payload);
} catch (error) {
  if (error instanceof MycelixError) {
    switch (error.code) {
      case ErrorCode.NotFound:
        console.log('Primitive not found');
        break;
      case ErrorCode.PhaseRestricted:
        console.log(`Not allowed in ${error.phase} phase`);
        break;
      case ErrorCode.RateLimited:
        console.log(`Retry after ${error.retryAfter}ms`);
        break;
      default:
        console.error(error.message);
    }
  }
}
```

## Connection Management

```typescript
// Manual connection control
await client.connect();
await client.disconnect();

// Connection events
client.on('connect', () => console.log('Connected'));
client.on('disconnect', (reason) => console.log('Disconnected:', reason));
client.on('reconnecting', (attempt) => console.log('Reconnecting:', attempt));
client.on('error', (error) => console.error('Error:', error));

// Check connection state
console.log(client.connected);  // boolean
console.log(client.state);      // 'connecting' | 'connected' | 'disconnected'
```

## React Integration

```typescript
import { MycelixProvider, useCycle, usePrimitive } from '@mycelix/sdk/react';

// Provider setup
function App() {
  return (
    <MycelixProvider url="wss://mycelix.example.com/ws" apiKey="mk_xxx">
      <Dashboard />
    </MycelixProvider>
  );
}

// Use cycle state
function CycleStatus() {
  const { phase, day, loading, error } = useCycle();

  if (loading) return <div>Loading...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return (
    <div>
      <p>Phase: {phase}</p>
      <p>Day: {day}/28</p>
    </div>
  );
}

// Use primitive
function PulseMonitor({ id }: { id: string }) {
  const { primitive, emissions, invoke, loading } = usePrimitive(id);

  return (
    <div>
      <h3>{primitive?.name}</h3>
      <p>Last emission: {emissions[0]?.timestamp}</p>
      <button onClick={() => invoke({ action: 'trigger' })}>
        Trigger
      </button>
    </div>
  );
}
```

## Testing

```typescript
import { createMockClient } from '@mycelix/sdk/testing';

describe('MyApp', () => {
  const mockClient = createMockClient({
    cycle: {
      phase: 'Surge',
      day: 10,
    },
    primitives: [
      { id: 'pulse-1', type: 'pulse', name: 'test' },
    ],
  });

  it('should handle surge phase', async () => {
    const result = await mockClient.cycle.status();
    expect(result.phase).toBe('Surge');
  });
});
```

## Next Steps

- [Python SDK](./python) - Python client
- [Go SDK](./go) - Go client
- [WebSocket API](../api/websocket) - Protocol reference
