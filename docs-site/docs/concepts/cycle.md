---
sidebar_position: 1
title: The 28-Day Cycle
---

# The 28-Day Cycle

At the heart of Mycelix lies the **28-day cycle** - a natural rhythm that governs system behavior, resource allocation, and primitive activation.

## Why 28 Days?

The 28-day cycle mirrors patterns found throughout nature:

- **Lunar cycles** - 29.5 days, rounded to 28 for simplicity
- **Cellular regeneration** - Many tissues renew on ~4 week cycles
- **Mycelial fruiting** - Mushroom networks follow lunar patterns

This isn't arbitrary. Systems that align with natural rhythms demonstrate:
- Reduced entropy accumulation
- More predictable resource patterns
- Natural points for reflection and adaptation

## The Four Phases

```
        Dawn (1-7)           Surge (8-14)
           ╱  ╲                 ╱  ╲
          ╱    ╲               ╱    ╲
    ─────●──────●─────────────●──────●─────
              ╲            ╱
               ╲          ╱
              Settle (15-21)     Rest (22-28)
```

### Dawn (Days 1-7)

**Theme**: Awakening, initialization, potential

During Dawn, the system:
- Initializes new primitives gently
- Prioritizes connection establishment
- Favors exploration over exploitation
- Accepts higher latency for stability

```typescript
// Dawn-optimized behavior
if (context.phase === 'Dawn') {
  // Take time to establish connections
  await this.connect({ timeout: '30s', retries: 10 });

  // Initialize with warmup period
  await this.warmup({ gradual: true });
}
```

**Best for**: New deployments, major updates, onboarding users

### Surge (Days 8-14)

**Theme**: Peak activity, maximum throughput, expansion

During Surge, the system:
- Operates at full capacity
- Minimizes latency at all costs
- Scales aggressively
- Prioritizes throughput over durability

```typescript
// Surge-optimized behavior
if (context.phase === 'Surge') {
  // Maximum parallelism
  await Promise.all(
    tasks.map(t => this.process(t, { priority: 'high' }))
  );

  // Aggressive caching
  this.cache.mode = 'aggressive';
}
```

**Best for**: Marketing campaigns, high-traffic events, batch processing

### Settle (Days 15-21)

**Theme**: Consolidation, pattern recognition, stability

During Settle, the system:
- Analyzes accumulated patterns
- Consolidates learnings
- Optimizes based on Surge data
- Prepares for Rest phase

```typescript
// Settle-optimized behavior
if (context.phase === 'Settle') {
  // Analyze patterns from Surge
  const patterns = await this.analyze({
    period: 'Surge',
    metrics: ['latency', 'errors', 'usage'],
  });

  // Apply optimizations
  await this.optimize(patterns);
}
```

**Best for**: Analytics, reporting, optimization, A/B test conclusions

### Rest (Days 22-28)

**Theme**: Reflection, pruning, regeneration

During Rest, the system:
- Reduces activity to minimum viable
- Prunes unused resources
- Performs maintenance operations
- Prepares for next cycle's Dawn

```typescript
// Rest-optimized behavior
if (context.phase === 'Rest') {
  // Minimal processing
  this.throttle({ maxConcurrency: 2 });

  // Cleanup unused resources
  await this.prune({ olderThan: '14d' });

  // Prepare for next cycle
  await this.prepareNextCycle();
}
```

**Best for**: Maintenance windows, deprecations, team retrospectives

## Cycle Configuration

### Setting the Cycle Start

```typescript
const mycelix = new Mycelix({
  cycle: {
    // When did this cycle begin?
    startDate: '2024-01-01',

    // Timezone for day calculations
    timezone: 'UTC',

    // Optional: custom phase lengths (must sum to 28)
    phases: {
      Dawn: 7,
      Surge: 7,
      Settle: 7,
      Rest: 7,
    },
  },
});
```

### Querying Cycle State

```typescript
// Current state
const phase = mycelix.currentPhase;  // 'Dawn' | 'Surge' | 'Settle' | 'Rest'
const day = mycelix.cycleDay;         // 1-28
const cycleNumber = mycelix.cycleNumber; // Which cycle since start

// Phase timing
const daysInPhase = mycelix.daysInCurrentPhase;  // 1-7
const daysUntilNextPhase = mycelix.daysUntilNextPhase;
const nextPhase = mycelix.nextPhase;

// Cycle timing
const cycleProgress = mycelix.cycleProgress;  // 0.0 - 1.0
const daysUntilNextCycle = mycelix.daysUntilNextCycle;
```

### Phase Change Events

```typescript
mycelix.on('phaseChange', (event) => {
  console.log(`Transitioning from ${event.from} to ${event.to}`);
  console.log(`Cycle ${event.cycleNumber}, Day ${event.day}`);
});

mycelix.on('cycleComplete', (event) => {
  console.log(`Cycle ${event.cycleNumber} complete`);
  console.log(`Starting cycle ${event.cycleNumber + 1}`);
});
```

## Phase-Aware Primitives

All Mycelix primitives can adapt to the current phase:

```typescript
const adaptivePulse = new Pulse({
  name: 'adaptive',

  // Different intervals per phase
  interval: {
    Dawn: '10m',
    Surge: '1m',
    Settle: '5m',
    Rest: '30m',
  },

  // Phase-conditional execution
  enabled: (phase) => phase !== 'Rest',

  emit: async (context) => {
    // Access phase in your logic
    if (context.phase === 'Surge') {
      return { priority: 'high', ...data };
    }
    return { priority: 'normal', ...data };
  },
});
```

## Best Practices

### 1. Align Deployments with Dawn

Major releases should target the Dawn phase when the system is most receptive to change.

### 2. Schedule Heavy Work for Surge

Batch jobs, data migrations, and intensive operations belong in Surge.

### 3. Analyze During Settle

Use Settle to review metrics, draw conclusions, and plan optimizations.

### 4. Maintain During Rest

Deprecations, cleanups, and breaking changes are safest during Rest.

### 5. Never Fight the Cycle

If the system wants to rest, let it. Forcing Surge-level activity during Rest leads to:
- Increased error rates
- Resource exhaustion
- Accumulated technical debt

## Advanced: Custom Cycles

For specialized use cases, you can define custom cycles:

```typescript
const mycelix = new Mycelix({
  cycle: {
    // 7-day sprint cycle
    type: 'custom',
    length: 7,
    phases: [
      { name: 'Plan', days: 1 },
      { name: 'Build', days: 4 },
      { name: 'Ship', days: 1 },
      { name: 'Retro', days: 1 },
    ],
  },
});
```

## Next Steps

- [Phases in Detail](./phases) - Deep dive into each phase
- [21 Primitives](./primitives) - Building blocks that understand the cycle
