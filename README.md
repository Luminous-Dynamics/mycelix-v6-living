# Mycelix v6.0 Living Protocol Layer

[![Crates.io](https://img.shields.io/crates/v/living-core.svg)](https://crates.io/crates/living-core)
[![npm](https://img.shields.io/npm/v/@mycelix/living-protocol-sdk.svg)](https://www.npmjs.com/package/@mycelix/living-protocol-sdk)
[![PyPI](https://img.shields.io/pypi/v/mycelix.svg)](https://pypi.org/project/mycelix/)
[![Go Reference](https://pkg.go.dev/badge/github.com/mycelix/mycelix-go.svg)](https://pkg.go.dev/github.com/mycelix/mycelix-go)
[![CI](https://github.com/mycelix/mycelix-v6-living/actions/workflows/ci.yml/badge.svg)](https://github.com/mycelix/mycelix-v6-living/actions/workflows/ci.yml)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Docker](https://img.shields.io/badge/docker-ghcr.io%2Fmycelix%2Fmycelix--v6--living-blue)](https://ghcr.io/mycelix/mycelix-v6-living)

A bioregional protocol implementation with 21 living primitives for decentralized coordination, consciousness integration, and regenerative governance.

## Overview

The Living Protocol Layer extends Mycelix with primitives inspired by biological systems, consciousness research, and regenerative design patterns. It operates on a 28-day metabolism cycle with 9 distinct phases.

```
┌─────────────────────────────────────────────────────────────────┐
│                    21 Living Primitives                         │
├─────────────┬─────────────┬─────────────┬─────────────┬─────────┤
│ Metabolism  │Consciousness│  Epistemics │  Relational │Structural│
├─────────────┼─────────────┼─────────────┼─────────────┼─────────┤
│ Composting  │ Temporal    │ Shadow      │ Entangled   │Resonance│
│ Wound       │ K-Vector    │ Integration │ Pairs       │Addressing│
│ Healing     │ Field       │ Negative    │ Eros        │Fractal  │
│ Metabolic   │ Interference│ Capability  │ Attractor   │Governance│
│ Trust       │ Collective  │ Silence     │ Liminality  │Morpho-  │
│ Kenosis     │ Dreaming    │ Signaling   │ Inter-      │genetic  │
│             │ Emergent    │ Beauty      │ Species     │Time     │
│             │ Personhood  │ Validity    │             │Crystal  │
│             │             │             │             │Mycelial │
│             │             │             │             │Compute  │
└─────────────┴─────────────┴─────────────┴─────────────┴─────────┘
```

## Quick Start

### Prerequisites

- Rust 1.75+ with `wasm32-unknown-unknown` target
- Node.js 18+ and npm
- (Optional) Holochain SDK via [holonix](https://developer.holochain.org/get-started/)
- (Optional) [Foundry](https://book.getfoundry.sh/) for Solidity

### Installation

```bash
# Clone the repository
git clone https://github.com/mycelix/mycelix-v6-living.git
cd mycelix-v6-living

# Build Rust crates
cargo build --release

# Build TypeScript SDK
cd sdk/typescript
npm install
npm run build
```

### Run the WebSocket Server

```bash
# Start with defaults (localhost:8888)
cargo run -p ws-server

# Custom host and port
cargo run -p ws-server -- --host 0.0.0.0 --port 9000

# With debug logging
cargo run -p ws-server -- --log-level debug

# With simulated time (for testing/development)
cargo run -p ws-server -- --simulated-time --time-acceleration 100
```

The server exposes:
- **WebSocket RPC** on port 8888 - JSON-RPC style API for querying cycle state
- **Health/Metrics HTTP** on port 8889 - `/health`, `/metrics`, `/state` endpoints

### Run with Docker

```bash
# Build and run
docker build -t mycelix-ws-server .
docker run -p 8888:8888 -p 8889:8889 mycelix-ws-server

# Or use docker-compose
docker-compose up -d

# Development mode (100x time acceleration)
docker-compose --profile dev up ws-server-dev
```

### Run Tests

```bash
# Run all Rust tests
cargo test --workspace --features full

# Run WebSocket server E2E tests
cargo test -p ws-server --test e2e

# Run with specific feature tiers
cargo test --workspace --features tier3-experimental
cargo test --workspace --features tier4-aspirational
```

### Using the TypeScript SDK

```typescript
import { LivingProtocolClient, CyclePhase } from '@mycelix/living-protocol-sdk';

// Connect to WebSocket RPC server
const client = await LivingProtocolClient.connect({
  url: 'ws://localhost:8888',
});

// Query cycle state
const state = await client.getCurrentState();
console.log(`Cycle ${state.cycleNumber}, Phase: ${state.currentPhase}`);

// Subscribe to phase transitions
client.onPhaseChange((event) => {
  console.log(`Phase changed: ${event.data.from} -> ${event.data.to}`);
});

// Check if operations are permitted
const canVote = await client.isOperationPermitted('vote');

// Get phase metrics
const metrics = await client.getPhaseMetrics(CyclePhase.Shadow);

// Clean disconnect
client.disconnect();
```

#### Running SDK Tests

```bash
cd sdk/typescript

# Run unit tests
npm test

# Run integration tests (requires running server)
# First: cargo run -p ws-server -- --port 9999 --simulated-time
npm run test:integration
```

## Architecture

### Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| Consensus | Holochain | Distributed validation, DHT storage |
| Smart Contracts | Solidity | Financial escrow, token burning |
| Core Logic | Rust | Primitive engines, cycle orchestration |
| Client SDK | TypeScript | Web/Node.js integration |

### Crate Structure

```
crates/
├── living-core/      # Shared types, events, errors
├── metabolism/       # Composting, Wounds, Trust, Kenosis
├── consciousness/    # K-Vectors, Fields, Dreams, Phi
├── epistemics/       # Shadow, Uncertainty, Silence, Beauty
├── relational/       # Entanglement, Attractors, Liminality
├── structural/       # Resonance, Fractals, Morphogenesis
├── cycle-engine/     # 28-day cycle orchestration
└── ws-server/        # WebSocket RPC server with health/metrics
```

### The 28-Day Metabolism Cycle

```
Day  1-3  │ Shadow           │ Surface suppressed content
Day  4-6  │ Composting       │ Decompose failed patterns
Day  7-9  │ Liminal          │ Threshold transitions
Day 10-12 │ NegativeCapability│ Hold in uncertainty
Day 13-15 │ Eros             │ Attractor field activation
Day 16-18 │ CoCreation       │ Entanglement formation
Day 19-21 │ Beauty           │ Aesthetic validation
Day 22-24 │ EmergentPersonhood│ Φ measurement
Day 25-28 │ Kenosis          │ Self-emptying commitments
```

## Feature Flags

```toml
[features]
default = []
tier3-experimental = ["consciousness/tier3-experimental"]
tier4-aspirational = ["consciousness/tier4-aspirational"]
full = ["tier3-experimental", "tier4-aspirational"]
```

| Tier | Features | Status |
|------|----------|--------|
| Tier 1-2 | Core primitives | Stable |
| Tier 3 | Collective Dreaming, Field Interference | Experimental |
| Tier 4 | Emergent Personhood, Inter-Species | Aspirational |

## Holochain Zomes

Build zomes for deployment:

```bash
# Enter holonix environment
nix develop

# Build all zomes
for zome in metabolism consciousness epistemics relational structural bridge; do
  cargo build --release --target wasm32-unknown-unknown \
    --manifest-path zomes/living-$zome/coordinator/Cargo.toml
done

# Package DNA
hc dna pack dna/
```

## Solidity Contracts

Three smart contracts handle on-chain financial operations:

| Contract | Purpose | Key Functions |
|----------|---------|---------------|
| `WoundEscrow.sol` | Healing-oriented escrow | `createWound`, `advancePhase`, `releaseEscrow` |
| `KenosisBurn.sol` | Reputation burning | `commitKenosis`, `executeKenosis` |
| `FractalDAO.sol` | Scale-invariant governance | `createPattern`, `submitProposal`, `vote` |

### Testing with Foundry

```bash
# Install dependencies
forge install

# Run tests
forge test

# Run with gas reporting
forge test --gas-report
```

## Gate System

The protocol enforces quality through three gate levels:

### Gate 1: Hard Invariants (Blocking)
- Wound phases advance forward only
- Kenosis max 20% per cycle
- K-Vector dimensions in [0.0, 1.0]

### Gate 2: Soft Constraints (Warning)
- Low-reputation dissent being suppressed
- Critical wound severity detected
- High epistemic novelty claim

### Gate 3: Network Health (Advisory)
- MATL integration thresholds
- Network Φ measurements
- Composting contribution tracking

## Integration with v5.2

The bridge zome enables integration with existing Mycelix v5.2:

```rust
// Fetch MATL score from v5.2
let matl = bridge::fetch_matl_score(agent_pubkey).await?;

// Convert slash to wound healing
let wound = bridge::intercept_slash(slash_event).await?;

// Attach beauty score to governance proposal
bridge::attach_beauty_score(proposal_hash, beauty_score).await?;
```

See [INTEGRATION.md](./INTEGRATION.md) for detailed integration architecture.

## API Reference

### WebSocket RPC Server

The WebSocket server provides a JSON-RPC style API:

**Request Format:**
```json
{ "id": "1", "method": "getCycleState", "params": {} }
```

**Response Format:**
```json
{ "id": "1", "result": { "cycleNumber": 1, "currentPhase": "Shadow", ... } }
```

**Available Methods:**

| Method | Params | Description |
|--------|--------|-------------|
| `getCycleState` | - | Get full cycle state |
| `getCurrentPhase` | - | Get current phase name |
| `getCycleNumber` | - | Get current cycle number |
| `getTransitionHistory` | - | Get phase transition history |
| `getPhaseMetrics` | `{ "phase": "Shadow" }` | Get metrics for a phase |
| `isOperationPermitted` | `{ "operation": "vote" }` | Check if operation is allowed |

**Health/Metrics Endpoints (HTTP):**

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check (returns `{"status":"healthy"}`) |
| `GET /metrics` | Server metrics (connections, messages, uptime) |
| `GET /state` | Current cycle state |

### Metabolism Module

```rust
use metabolism::{CompostingEngine, WoundHealingService, KenosisEngine};

// Start composting
let engine = CompostingEngine::new(config, event_bus);
engine.start_composting(entity_type, entity_id, reason)?;

// Wound healing
let wound = WoundHealingService::new(event_bus);
wound.create_wound(agent, severity, cause)?;
wound.advance_phase(wound_id)?;

// Kenosis (self-emptying)
let kenosis = KenosisEngine::new(config, event_bus);
kenosis.commit_kenosis(agent, release_percentage)?; // max 20%
```

### Consciousness Module

```rust
use consciousness::{TemporalKVectorService, EmergentPersonhoodService};

// Record K-Vector snapshot
let kvec = TemporalKVectorService::new();
kvec.record_snapshot(agent, dimensions)?;
let velocity = kvec.compute_velocity(agent)?;

// Measure network Phi
let phi = EmergentPersonhoodService::new();
let measurement = phi.measure_phi(agent_subset)?;
```

### Cycle Engine

```rust
use cycle_engine::{CycleEngineBuilder, CycleScheduler};

// Build engine with all handlers
let engine = CycleEngineBuilder::new()
    .with_config(config)
    .with_simulated_time(86400.0) // 1 day = 1 second
    .build();

// Run with scheduler
let scheduler = CycleScheduler::new(engine, 1)
    .on_events(|events| println!("{:?}", events));

scheduler.run().await?;
```

## Development

### Project Structure

```
mycelix-v6-living/
├── crates/           # Rust libraries
├── zomes/            # Holochain zomes
├── contracts/        # Solidity contracts
├── sdk/typescript/   # TypeScript SDK
├── examples/         # Example integrations
├── tests/            # Test suites
├── benches/          # Benchmarks
└── docs/             # Documentation
```

### Running Benchmarks

```bash
cargo bench --features full
```

### Code Style

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --workspace --features full
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-primitive`)
3. Commit changes (`git commit -m 'Add amazing primitive'`)
4. Push to branch (`git push origin feature/amazing-primitive`)
5. Open a Pull Request

## License

AGPL-3.0-or-later

## Resources

- [Living Protocol Specification](./docs/SPECIFICATION.md)
- [Integration Guide](./INTEGRATION.md)
- [API Documentation](./docs/API.md)
- [Status Report](./STATUS_REPORT.md)

## Acknowledgments

Inspired by:
- Holochain's agent-centric architecture
- Integrated Information Theory (IIT)
- Regenerative design patterns
- Mycelial network intelligence
- Jungian shadow integration
- Keats' negative capability
