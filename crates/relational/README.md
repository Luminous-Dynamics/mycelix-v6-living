# relational

Relational Field primitives for Mycelix Living Protocol.

## Overview

This crate implements relational dynamics between agents:

- **Entangled Pairs**: Quantum-inspired correlation between agents
- **Eros/Attractor Fields**: Desire-based coordination patterns
- **Liminality**: Threshold states and transitions
- **Inter-Species**: Communication across different agent types

## Installation

```toml
[dependencies]
relational = "0.6"
```

## Usage

```rust
use relational::{EntangledPair, AttractorField, LiminalSpace};

// Create entangled pair
let pair = EntangledPair::new(agent_a, agent_b);
pair.correlate()?;

// Define attractor field
let field = AttractorField::new(center, strength);
let pull = field.compute_attraction(agent)?;

// Enter liminal space
let liminal = LiminalSpace::new();
liminal.enter(agent, threshold)?;
```

## Relational Phases

- **Eros Phase (Day 13-15)**: Attractor fields most active
- **Co-Creation Phase (Day 16-18)**: Entanglement formation
- **Liminal Phase (Day 7-9)**: Threshold transitions

## License

AGPL-3.0-or-later
