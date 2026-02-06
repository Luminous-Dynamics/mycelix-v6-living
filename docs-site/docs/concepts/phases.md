---
sidebar_position: 2
title: Phases in Detail
---

# Phases in Detail

Each phase of the 28-day cycle has distinct characteristics, optimal use cases, and configuration options.

## Phase Overview

| Phase | Days | Energy | Focus | System Behavior |
|-------|------|--------|-------|-----------------|
| Dawn | 1-7 | Rising | Initialization | Gentle, exploratory |
| Surge | 8-14 | Peak | Performance | Aggressive, optimized |
| Settle | 15-21 | Declining | Analysis | Consolidating, reflective |
| Rest | 22-28 | Minimal | Recovery | Minimal, regenerative |

## Dawn Phase (Days 1-7)

### Characteristics

Dawn is the phase of **potential and preparation**. Like the first light of day, systems gradually come online with gentle initialization.

```typescript
// Dawn phase configuration
const dawnConfig = {
  // Connection behavior
  connections: {
    timeout: '30s',      // Patient connection attempts
    retries: 10,         // Many retry attempts
    backoff: 'gentle',   // Slow exponential backoff
  },

  // Resource allocation
  resources: {
    maxConcurrency: 0.5, // 50% of max capacity
    warmupPeriod: '2h',  // Gradual ramp-up
  },

  // Error handling
  errors: {
    tolerance: 'high',   // Accept more failures
    retry: true,         // Always retry
    circuit: 'lenient',  // Slow to open circuit breakers
  },
};
```

### System Behavior

During Dawn, primitives exhibit:

- **Exploratory connections**: The system tries multiple paths to establish optimal routes
- **Gentle initialization**: Resources warm up gradually, avoiding cold-start spikes
- **High fault tolerance**: Transient errors are expected and handled gracefully
- **Discovery mode**: New nodes are welcomed, network topology evolves

### Best Practices

```typescript
// Dawn-appropriate operations
await mycelix.duringDawn(async (ctx) => {
  // Establish new connections
  await ctx.discover({ broadcast: true });

  // Initialize caches with warm data
  await ctx.cache.warmup(['users', 'config', 'sessions']);

  // Run health checks
  const health = await ctx.healthCheck({ comprehensive: true });

  // Set up monitoring baselines
  await ctx.metrics.baseline(health);
});
```

### What to Avoid

- Large batch operations
- Aggressive timeouts
- High-throughput testing
- Production load before Day 3

---

## Surge Phase (Days 8-14)

### Characteristics

Surge is the phase of **maximum output**. The system operates at peak capacity with minimal overhead.

```typescript
// Surge phase configuration
const surgeConfig = {
  // Connection behavior
  connections: {
    timeout: '5s',       // Fast timeout
    retries: 2,          // Quick fail
    backoff: 'none',     // Immediate retry
  },

  // Resource allocation
  resources: {
    maxConcurrency: 1.0, // 100% capacity
    scaling: 'aggressive', // Scale up immediately
  },

  // Error handling
  errors: {
    tolerance: 'low',    // Fail fast
    retry: false,        // Let client retry
    circuit: 'strict',   // Protect the system
  },
};
```

### System Behavior

During Surge, primitives exhibit:

- **Maximum throughput**: All resources available, minimal queueing
- **Aggressive optimization**: Hot paths are prioritized, cold paths deferred
- **Strict timeouts**: Slow operations are terminated quickly
- **Horizontal scaling**: Nodes spin up rapidly to meet demand

### Best Practices

```typescript
// Surge-appropriate operations
await mycelix.duringSurge(async (ctx) => {
  // Process at maximum speed
  await ctx.process(items, {
    parallel: true,
    batchSize: 1000,
    timeout: '5s',
  });

  // Aggressive caching
  ctx.cache.mode = 'write-through';
  ctx.cache.ttl = '1h';

  // Real-time metrics
  ctx.metrics.interval = '10s';
});
```

### What to Avoid

- Major infrastructure changes
- Database migrations
- New feature deployments
- Experimental code paths

---

## Settle Phase (Days 15-21)

### Characteristics

Settle is the phase of **consolidation and analysis**. The system processes what happened during Surge.

```typescript
// Settle phase configuration
const settleConfig = {
  // Connection behavior
  connections: {
    timeout: '15s',      // Moderate patience
    retries: 5,          // Reasonable retry
    backoff: 'standard', // Normal backoff
  },

  // Resource allocation
  resources: {
    maxConcurrency: 0.7, // 70% capacity
    scaling: 'conservative', // Scale down gradually
  },

  // Analysis mode
  analysis: {
    enabled: true,
    patterns: ['usage', 'errors', 'latency'],
    window: 'Surge', // Analyze Surge data
  },
};
```

### System Behavior

During Settle, primitives exhibit:

- **Pattern recognition**: Accumulated data is analyzed for trends
- **Gradual wind-down**: Activity decreases smoothly
- **Consolidation**: Temporary data becomes permanent or is discarded
- **Optimization**: System tunes itself based on Surge learnings

### Best Practices

```typescript
// Settle-appropriate operations
await mycelix.duringSettle(async (ctx) => {
  // Analyze Surge performance
  const analysis = await ctx.analyze({
    phase: 'Surge',
    metrics: ['p99_latency', 'error_rate', 'throughput'],
  });

  // Consolidate learnings
  await ctx.optimize({
    cachePolicy: analysis.cacheRecommendations,
    routing: analysis.routingOptimizations,
  });

  // Generate reports
  await ctx.report({
    type: 'cycle-summary',
    include: ['metrics', 'patterns', 'recommendations'],
  });
});
```

### What to Avoid

- High-throughput operations
- Time-sensitive processing
- New feature launches
- Aggressive scaling

---

## Rest Phase (Days 22-28)

### Characteristics

Rest is the phase of **recovery and preparation**. Minimal activity, maximum reflection.

```typescript
// Rest phase configuration
const restConfig = {
  // Connection behavior
  connections: {
    timeout: '60s',      // Very patient
    retries: 3,          // Minimal retry
    backoff: 'very-slow', // Long waits
  },

  // Resource allocation
  resources: {
    maxConcurrency: 0.2, // 20% capacity
    scaling: 'none',     // No scaling
  },

  // Maintenance mode
  maintenance: {
    enabled: true,
    operations: ['prune', 'vacuum', 'archive'],
  },
};
```

### System Behavior

During Rest, primitives exhibit:

- **Minimal activity**: Only essential operations proceed
- **Maintenance mode**: Cleanup and optimization tasks run
- **Reflection**: System logs are analyzed, learnings extracted
- **Preparation**: Next cycle is planned and staged

### Best Practices

```typescript
// Rest-appropriate operations
await mycelix.duringRest(async (ctx) => {
  // Minimal processing
  ctx.throttle({ maxOps: 100 });

  // Cleanup operations
  await ctx.prune({
    staleData: '30d',
    orphanedRecords: true,
    tempFiles: true,
  });

  // Archive historical data
  await ctx.archive({
    olderThan: '90d',
    destination: 's3://archive',
  });

  // Prepare next cycle
  await ctx.prepareNextCycle({
    preload: ['config', 'schemas'],
    warmCaches: false, // Dawn will warm them
  });
});
```

### What to Avoid

- Any non-essential operations
- User-facing feature work
- Performance testing
- Aggressive monitoring

---

## Phase Transitions

### Transition Events

```typescript
mycelix.on('phaseWillChange', async (event) => {
  // Prepare for transition
  console.log(`Preparing to exit ${event.from}, entering ${event.to}`);

  // Flush buffers before leaving Surge
  if (event.from === 'Surge') {
    await mycelix.flush();
  }
});

mycelix.on('phaseDidChange', async (event) => {
  // React to completed transition
  console.log(`Now in ${event.to} phase`);

  // Start warmup when entering Dawn
  if (event.to === 'Dawn') {
    await mycelix.warmup();
  }
});
```

### Transition Windows

Transitions occur at midnight UTC by default, but can be configured:

```typescript
const mycelix = new Mycelix({
  cycle: {
    transitionTime: '03:00', // 3 AM
    transitionZone: 'America/New_York',
    transitionWindow: '1h', // Gradual over 1 hour
  },
});
```

## Phase-Specific Metrics

Each phase tracks different metrics by priority:

| Metric | Dawn | Surge | Settle | Rest |
|--------|------|-------|--------|------|
| Latency (p99) | Low | High | Medium | Low |
| Throughput | Low | High | Medium | Low |
| Error Rate | High | High | Medium | Low |
| Memory Usage | Medium | High | Medium | Low |
| CPU Usage | Low | High | Medium | Low |
| Connection Count | High | Medium | Low | Low |

## Next Steps

- [21 Primitives](./primitives) - Building blocks that understand phases
- [Server Configuration](../server/configuration) - Configure phase behavior
