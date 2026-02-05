# Mycelix v6.0 Living Protocol Layer

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

### Run Tests

```bash
# Run all Rust tests (426 tests)
cargo test --workspace --features full

# Run with specific feature tiers
cargo test --workspace --features tier3-experimental
cargo test --workspace --features tier4-aspirational
```

### Using the TypeScript SDK

```typescript
import {
  MetabolismClient,
  ConsciousnessClient,
  CycleClient
} from '@mycelix/living-protocol-sdk';

// Connect to Holochain
const client = await AppAgentWebsocket.connect('ws://localhost:8888', 'mycelix');

// Create a metabolism client
const metabolism = new MetabolismClient(client, 'living_metabolism');

// Start composting a failed proposal
await metabolism.startComposting({
  entityType: 'FailedProposal',
  entityId: 'proposal-123',
  reason: 'Quorum not reached'
});

// Create a wound from a protocol violation
const wound = await metabolism.createWound({
  severity: 'Moderate',
  cause: 'Consensus violation',
  escrowAmount: 1000
});

// Advance through healing phases
await metabolism.advanceWoundPhase(wound.id); // Hemostasis -> Inflammation
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
└── cycle-engine/     # 28-day cycle orchestration
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
