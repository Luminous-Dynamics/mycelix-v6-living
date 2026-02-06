# Living Protocol SDK Examples

This directory contains example applications demonstrating how to use the Living Protocol TypeScript SDK.

## Examples

### 1. React Dashboard (`dashboard/`)

A real-time monitoring dashboard built with React, TypeScript, and Vite.

**Features:**
- Visual 9-phase timeline with progress indicator
- Current cycle status display
- Live network metrics (agents, spectral K, metabolic trust)
- Phase transition history log
- Real-time event stream

**Quick Start:**
```bash
cd dashboard
npm install
npm run dev
```

Open http://localhost:3000 in your browser.

[View Dashboard README](./dashboard/README.md)

### 2. CLI Tool (`cli/`)

A command-line interface for monitoring and querying the Living Protocol.

**Commands:**
- `status` - Show current cycle state
- `watch` - Stream live events
- `history` - View phase transitions
- `metrics` - Display network metrics
- `phases` - List all cycle phases

**Quick Start:**
```bash
cd cli
npm install
npm run build
npm start -- status
```

[View CLI README](./cli/README.md)

### 3. Simple dApp (`simple-dapp/`)

A minimal example showing basic SDK integration.

## Prerequisites

All examples require:

1. **Node.js 18+** - JavaScript runtime
2. **Running Living Protocol Server** - WebSocket server at `ws://localhost:8888/ws` (or configure a custom URL)
3. **TypeScript SDK** - The examples link to the local SDK at `../../sdk/typescript`

## The Living Protocol Cycle

The Living Protocol operates on a **28-day lunar metabolism cycle** with 9 distinct phases:

```
Day 1-2    : Shadow              - Integration of suppressed content
Day 3-7    : Composting          - Decomposition and nutrient extraction
Day 8-10   : Liminal             - Threshold state between identities
Day 11-13  : Negative Capability - Holding uncertainty without resolution
Day 14-17  : Eros                - Attraction and creative tension
Day 18-24  : Co-Creation         - Collaborative emergence
Day 25-26  : Beauty              - Aesthetic validation
Day 27     : Emergent Personhood - Network consciousness assessment
Day 28     : Kenosis             - Voluntary release and emptying
```

## SDK Quick Reference

### Connecting to the Protocol

```typescript
import { LivingProtocolClient } from '@mycelix/living-protocol-sdk';

// Connect to server
const client = await LivingProtocolClient.connect({
  url: 'ws://localhost:8888/ws',
});

// Get current cycle state
const state = await client.getCurrentState();
console.log(`Cycle ${state.cycleNumber}, Phase: ${state.currentPhase}`);

// Subscribe to events
client.onPhaseChange((event) => {
  console.log(`Transitioned from ${event.data.from} to ${event.data.to}`);
});

// Clean up
client.disconnect();
```

### Available Event Types

The SDK provides type-safe subscriptions to various protocol events:

```typescript
// Phase changes
client.onPhaseChange(callback);
client.onCycleStart(callback);

// Metabolism
client.onWoundCreated(callback);
client.onWoundAdvanced(callback);
client.onKenosis(callback);

// Relational
client.onEntanglement(callback);
client.onShadowSurfaced(callback);

// Custom filter
client.subscribeWithFilter({
  eventTypes: ['WoundCreated', 'EntanglementFormed'],
  phases: [CyclePhase.Eros, CyclePhase.CoCreation],
}, callback);
```

### Network Metrics

```typescript
// Get current metrics
const metrics = await client.getPhaseMetrics(state.currentPhase);

console.log({
  activeAgents: metrics.activeAgents,
  spectralK: metrics.spectralK,
  metabolicTrust: metrics.meanMetabolicTrust,
  activeWounds: metrics.activeWounds,
  entangledPairs: metrics.entangledPairs,
});
```

### Phase Operations

```typescript
// Check if operations are permitted
const canVote = await client.isOperationPermitted('vote');
const isFinancialBlocked = await client.isFinancialBlocked();

// Get time remaining in phase
const remainingMs = await client.getTimeRemaining();
```

## Project Structure

```
examples/
├── README.md              # This file
├── dashboard/             # React dashboard example
│   ├── src/
│   │   ├── App.tsx
│   │   ├── components/
│   │   │   ├── CycleStatus.tsx
│   │   │   ├── PhaseTimeline.tsx
│   │   │   ├── TransitionHistory.tsx
│   │   │   └── MetricsPanel.tsx
│   │   └── hooks/
│   │       └── useLivingProtocol.ts
│   ├── index.html
│   ├── vite.config.ts
│   └── package.json
├── cli/                   # CLI tool example
│   ├── src/
│   │   └── index.ts
│   └── package.json
└── simple-dapp/           # Minimal example
```

## Running the Examples

### Option 1: With a Running Server

Start the Living Protocol server:

```bash
# From project root
cargo run --release -- --ws-port 8888
```

Then run any example.

### Option 2: Mock/Demo Mode

For testing without a server, you can modify the examples to use mock data (not included by default).

## Troubleshooting

### Connection Errors

If you see "Connection failed" or "WebSocket error":

1. Ensure the server is running: `cargo run`
2. Check the WebSocket URL matches your server configuration
3. Verify no firewall is blocking port 8888

### TypeScript Errors

If you see TypeScript errors about the SDK:

1. Ensure the SDK is built: `cd ../../sdk/typescript && npm run build`
2. Run `npm install` in the example directory

### Module Resolution

If you see "Cannot find module" errors:

1. Check that `@mycelix/living-protocol-sdk` is linked correctly in `package.json`
2. Run `npm install` to ensure dependencies are installed

## Contributing

When adding new examples:

1. Create a new directory under `examples/`
2. Include a `README.md` with setup instructions
3. Link to the SDK using `"@mycelix/living-protocol-sdk": "file:../../sdk/typescript"`
4. Update this README with the new example

## License

AGPL-3.0-or-later
