# metabolism

Metabolism Engine for Mycelix Living Protocol: Composting, Wound Healing, Metabolic Trust, and Kenosis.

## Overview

The metabolism crate implements the first pillar of the Living Protocol:

- **Composting**: Transform failed patterns into nutrients for new growth
- **Wound Healing**: Multi-phase healing process for organizational injuries
- **Metabolic Trust**: Contribution-based trust that must be renewed
- **Kenosis**: Self-emptying commitments that prevent accumulation

## Installation

```toml
[dependencies]
metabolism = "0.6"
```

## Usage

```rust
use metabolism::{CompostingEngine, WoundHealingService, KenosisEngine};

// Start composting a failed pattern
let engine = CompostingEngine::new(config, event_bus);
engine.start_composting(entity_type, entity_id, reason)?;

// Create and heal a wound
let wound = WoundHealingService::new(event_bus);
let wound_id = wound.create_wound(agent, severity, cause)?;
wound.advance_phase(wound_id)?;

// Commit to kenosis (max 20% per cycle)
let kenosis = KenosisEngine::new(config, event_bus);
kenosis.commit_kenosis(agent, 0.15)?;
```

## The 28-Day Metabolism Cycle

```
Day  1-3  | Shadow           | Surface suppressed content
Day  4-6  | Composting       | Decompose failed patterns
Day  7-9  | Liminal          | Threshold transitions
...
Day 25-28 | Kenosis          | Self-emptying commitments
```

## License

AGPL-3.0-or-later
