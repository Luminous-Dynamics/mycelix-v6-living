---
sidebar_position: 3
title: 21 Living Primitives
---

# 21 Living Primitives

Mycelix provides 21 building blocks called **primitives** - each understanding time, phase, and context. They are grouped into seven families of three.

## Primitive Families

```
Communication    │  Coordination   │  Computation
─────────────────┼─────────────────┼─────────────────
Pulse            │  Thread         │  Spore
Signal           │  Weave          │  Bloom
Echo             │  Mesh           │  Fruit

Storage          │  Flow           │  Awareness
─────────────────┼─────────────────┼─────────────────
Root             │  Stream         │  Sense
Mycelium         │  Pool           │  Dream
Archive          │  Gate           │  Wake

                 │  Meta           │
                 ┼─────────────────┼
                 │  Cycle          │
                 │  Phase          │
                 │  Rhythm         │
```

---

## Communication Family

Primitives for sending and receiving messages.

### Pulse

Emits periodic signals at phase-aware intervals.

```typescript
import { Pulse } from '@mycelix/core';

const heartbeat = new Pulse({
  name: 'heartbeat',
  interval: {
    Dawn: '10s',
    Surge: '1s',
    Settle: '5s',
    Rest: '30s',
  },
  emit: async (ctx) => ({
    status: 'alive',
    phase: ctx.phase,
    uptime: process.uptime(),
  }),
});
```

**Use cases**: Health checks, status broadcasts, periodic updates

### Signal

One-time directional message with guaranteed delivery.

```typescript
import { Signal } from '@mycelix/core';

const alert = new Signal({
  name: 'alert',
  priority: (ctx) => ctx.phase === 'Surge' ? 'high' : 'normal',
  deliver: async (payload, ctx) => {
    await ctx.send(payload.target, payload.message);
    return { delivered: true };
  },
});
```

**Use cases**: Alerts, notifications, point-to-point messaging

### Echo

Broadcasts message and collects responses from all listeners.

```typescript
import { Echo } from '@mycelix/core';

const discovery = new Echo({
  name: 'discover',
  timeout: (ctx) => ctx.phase === 'Dawn' ? '30s' : '5s',
  broadcast: async (query, ctx) => {
    const responses = await ctx.broadcast(query);
    return responses.filter(r => r.matches);
  },
});
```

**Use cases**: Service discovery, consensus gathering, health aggregation

---

## Coordination Family

Primitives for orchestrating work across nodes.

### Thread

Handles incoming requests with phase-aware processing.

```typescript
import { Thread } from '@mycelix/core';

const processor = new Thread({
  name: 'processor',
  concurrency: (ctx) => ({
    Dawn: 2,
    Surge: 10,
    Settle: 5,
    Rest: 1,
  }[ctx.phase]),
  handle: async (request, ctx) => {
    const result = await process(request);
    return { result, processedAt: ctx.now };
  },
});
```

**Use cases**: Request handling, job processing, API endpoints

### Weave

Combines multiple threads into coordinated workflows.

```typescript
import { Weave } from '@mycelix/core';

const pipeline = new Weave({
  name: 'etl-pipeline',
  threads: ['extract', 'transform', 'load'],
  strategy: (ctx) => ctx.phase === 'Surge' ? 'parallel' : 'sequential',
  weave: async (input, threads, ctx) => {
    let data = input;
    for (const thread of threads) {
      data = await thread.handle(data, ctx);
    }
    return data;
  },
});
```

**Use cases**: Pipelines, sagas, multi-step workflows

### Mesh

Creates a self-organizing network of connected primitives.

```typescript
import { Mesh } from '@mycelix/core';

const cluster = new Mesh({
  name: 'compute-mesh',
  topology: 'auto',
  rebalance: (ctx) => ctx.phase === 'Settle',
  connect: async (nodes, ctx) => {
    return nodes.map(n => ({
      node: n,
      connections: ctx.optimalConnections(n),
    }));
  },
});
```

**Use cases**: Distributed computing, peer networks, load distribution

---

## Computation Family

Primitives for executing and evolving logic.

### Spore

Encapsulates portable computation that can migrate between nodes.

```typescript
import { Spore } from '@mycelix/core';

const task = new Spore({
  name: 'analysis-task',
  portable: true,
  migrate: (ctx) => ctx.phase === 'Rest' ? 'archive-node' : null,
  compute: async (input, ctx) => {
    const result = await analyze(input);
    return { result, computedDuring: ctx.phase };
  },
});
```

**Use cases**: Mobile computation, distributed tasks, node migration

### Bloom

Represents computation that grows and evolves over cycles.

```typescript
import { Bloom } from '@mycelix/core';

const model = new Bloom({
  name: 'recommendation-model',
  evolve: (ctx) => ctx.phase === 'Settle',
  grow: async (currentState, ctx) => {
    if (ctx.phase === 'Settle') {
      const patterns = await ctx.analyze('Surge');
      return { ...currentState, patterns };
    }
    return currentState;
  },
});
```

**Use cases**: ML models, adaptive algorithms, evolving configurations

### Fruit

Final output of a Bloom, ready for consumption.

```typescript
import { Fruit } from '@mycelix/core';

const output = new Fruit({
  name: 'recommendations',
  source: 'recommendation-model',
  harvest: (ctx) => ctx.phase === 'Dawn',
  ripen: async (bloom, ctx) => {
    const model = await bloom.current();
    return {
      recommendations: model.patterns.top(10),
      harvestedCycle: ctx.cycleNumber,
    };
  },
});
```

**Use cases**: Model outputs, report generation, computed results

---

## Storage Family

Primitives for persisting and retrieving data.

### Root

Fast, local storage for frequently accessed data.

```typescript
import { Root } from '@mycelix/core';

const cache = new Root({
  name: 'session-cache',
  ttl: (ctx) => ctx.phase === 'Surge' ? '1h' : '4h',
  store: async (key, value, ctx) => {
    await ctx.local.set(key, value);
  },
  retrieve: async (key, ctx) => {
    return ctx.local.get(key);
  },
});
```

**Use cases**: Caching, session storage, hot data

### Mycelium

Distributed storage that spans the entire network.

```typescript
import { Mycelium } from '@mycelix/core';

const distributed = new Mycelium({
  name: 'shared-state',
  replication: 3,
  consistency: (ctx) => ctx.phase === 'Surge' ? 'eventual' : 'strong',
  spread: async (key, value, ctx) => {
    await ctx.distributed.replicate(key, value);
  },
});
```

**Use cases**: Distributed state, shared configuration, cross-node data

### Archive

Long-term storage for historical data.

```typescript
import { Archive } from '@mycelix/core';

const history = new Archive({
  name: 'event-archive',
  compress: true,
  archive: (ctx) => ctx.phase === 'Rest',
  store: async (events, ctx) => {
    await ctx.cold.append(events, { cycle: ctx.cycleNumber });
  },
});
```

**Use cases**: Audit logs, historical data, compliance records

---

## Flow Family

Primitives for managing data movement.

### Stream

Continuous flow of data with backpressure handling.

```typescript
import { Stream } from '@mycelix/core';

const events = new Stream({
  name: 'event-stream',
  buffer: (ctx) => ctx.phase === 'Surge' ? 10000 : 1000,
  flow: async function* (source, ctx) {
    for await (const event of source) {
      yield transform(event);
    }
  },
});
```

**Use cases**: Event processing, real-time data, log aggregation

### Pool

Manages a pool of reusable resources.

```typescript
import { Pool } from '@mycelix/core';

const connections = new Pool({
  name: 'db-pool',
  size: (ctx) => ({
    Dawn: 5,
    Surge: 20,
    Settle: 10,
    Rest: 2,
  }[ctx.phase]),
  create: async () => createConnection(),
  destroy: async (conn) => conn.close(),
});
```

**Use cases**: Connection pools, worker pools, resource management

### Gate

Controls flow with phase-aware admission.

```typescript
import { Gate } from '@mycelix/core';

const rateLimit = new Gate({
  name: 'api-gate',
  limit: (ctx) => ({
    Dawn: 100,
    Surge: 1000,
    Settle: 500,
    Rest: 50,
  }[ctx.phase]),
  admit: async (request, ctx) => {
    return ctx.remaining > 0;
  },
});
```

**Use cases**: Rate limiting, circuit breaking, admission control

---

## Awareness Family

Primitives for sensing and responding to environment.

### Sense

Monitors environment and emits observations.

```typescript
import { Sense } from '@mycelix/core';

const monitor = new Sense({
  name: 'system-monitor',
  interval: (ctx) => ctx.phase === 'Surge' ? '10s' : '1m',
  observe: async (ctx) => ({
    cpu: process.cpuUsage(),
    memory: process.memoryUsage(),
    phase: ctx.phase,
  }),
});
```

**Use cases**: Monitoring, observability, health sensing

### Dream

Background processing that runs during Rest phase.

```typescript
import { Dream } from '@mycelix/core';

const optimizer = new Dream({
  name: 'nightly-optimization',
  when: 'Rest',
  dream: async (ctx) => {
    // Runs only during Rest phase
    await ctx.optimize('indexes');
    await ctx.compact('storage');
    await ctx.analyze('patterns');
  },
});
```

**Use cases**: Background jobs, optimization, maintenance

### Wake

Triggers actions on phase transitions.

```typescript
import { Wake } from '@mycelix/core';

const startup = new Wake({
  name: 'dawn-startup',
  on: 'Dawn',
  wake: async (ctx) => {
    await ctx.warmCaches();
    await ctx.checkConnections();
    await ctx.announcePresence();
  },
});
```

**Use cases**: Initialization, transitions, scheduled actions

---

## Meta Family

Primitives for understanding the cycle itself.

### Cycle

Represents the full 28-day cycle.

```typescript
import { Cycle } from '@mycelix/core';

const cycle = new Cycle({
  name: 'main-cycle',
  start: '2024-01-01',
  onComplete: async (ctx) => {
    await ctx.archive(ctx.cycleNumber);
    await ctx.report('cycle-summary');
  },
});
```

**Use cases**: Cycle management, lifecycle hooks

### Phase

Represents a single phase within the cycle.

```typescript
import { Phase } from '@mycelix/core';

const surge = new Phase({
  name: 'surge-phase',
  phase: 'Surge',
  onEnter: async (ctx) => {
    await ctx.scale('up');
  },
  onExit: async (ctx) => {
    await ctx.scale('normal');
  },
});
```

**Use cases**: Phase-specific logic, transition hooks

### Rhythm

Defines custom timing patterns within phases.

```typescript
import { Rhythm } from '@mycelix/core';

const workday = new Rhythm({
  name: 'business-hours',
  pattern: '9-17 Mon-Fri',
  during: async (ctx) => {
    // Active during business hours
    ctx.capacity = 1.0;
  },
  outside: async (ctx) => {
    // Reduced capacity outside
    ctx.capacity = 0.3;
  },
});
```

**Use cases**: Business hours, custom schedules, time-based behavior

---

## Primitive Composition

Primitives can be composed to build complex systems:

```typescript
import { compose, Pulse, Thread, Stream, Gate } from '@mycelix/core';

const api = compose([
  new Gate({ name: 'rate-limit', limit: 1000 }),
  new Thread({ name: 'handler', concurrency: 10 }),
  new Stream({ name: 'events', buffer: 5000 }),
  new Pulse({ name: 'health', interval: '30s' }),
]);
```

## Next Steps

- [Server Configuration](../server/configuration) - Configure primitive behavior
- [WebSocket API](../api/websocket) - Real-time primitive access
