# consciousness

Consciousness Field primitives for Mycelix Living Protocol.

## Overview

This crate implements consciousness-related primitives:

- **Temporal K-Vector**: Multi-dimensional state tracking over time
- **Field Interference**: Awareness field interactions between agents
- **Collective Dreaming**: Shared imaginative spaces (Tier 3)
- **Emergent Personhood**: Network-wide consciousness metrics (Tier 4)

## Installation

```toml
[dependencies]
consciousness = "0.6"
```

## Feature Flags

```toml
[dependencies]
# Core features only
consciousness = "0.6"

# Include experimental features
consciousness = { version = "0.6", features = ["tier3-experimental"] }

# Include all features
consciousness = { version = "0.6", features = ["full"] }
```

## Usage

```rust
use consciousness::{TemporalKVectorService, KVector};

// Record K-Vector snapshot
let kvec = TemporalKVectorService::new();
kvec.record_snapshot(agent, dimensions)?;

// Compute velocity of change
let velocity = kvec.compute_velocity(agent)?;

// Measure coherence
let coherence = kvec.measure_coherence(agent_subset)?;
```

## Tier System

| Tier | Features | Status |
|------|----------|--------|
| Tier 1-2 | K-Vector, Field Interference | Stable |
| Tier 3 | Collective Dreaming | Experimental |
| Tier 4 | Emergent Personhood (Phi) | Aspirational |

## License

AGPL-3.0-or-later
