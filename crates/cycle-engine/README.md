# cycle-engine

Metabolism Cycle State Machine orchestrator for the Mycelix Living Protocol.

## Overview

This crate provides the core orchestration engine that coordinates all Living Protocol primitives through the 28-day metabolism cycle:

- **Phase Management**: Automatic phase transitions based on time
- **Event Coordination**: Routes events between subsystems
- **Gate Enforcement**: Validates operations against phase rules
- **Telemetry**: Optional OpenTelemetry integration

## Installation

```toml
[dependencies]
cycle-engine = "0.6"
```

## Feature Flags

```toml
[dependencies]
# Core engine only
cycle-engine = "0.6"

# With telemetry support
cycle-engine = { version = "0.6", features = ["telemetry"] }

# With experimental features
cycle-engine = { version = "0.6", features = ["tier3-experimental"] }

# All features
cycle-engine = { version = "0.6", features = ["full", "telemetry"] }
```

## Usage

```rust
use cycle_engine::{CycleEngineBuilder, CycleScheduler};

// Build the engine
let engine = CycleEngineBuilder::new()
    .with_config(config)
    .with_simulated_time(86400.0)  // 1 day = 1 second (for testing)
    .build();

// Create scheduler
let scheduler = CycleScheduler::new(engine, tick_interval_ms)
    .on_events(|events| {
        for event in events {
            println!("Event: {:?}", event);
        }
    });

// Run the cycle
scheduler.run().await?;
```

## The 28-Day Metabolism Cycle

```
Day  1-3  | Shadow           | Surface suppressed content
Day  4-6  | Composting       | Decompose failed patterns
Day  7-9  | Liminal          | Threshold transitions
Day 10-12 | NegativeCapability| Hold in uncertainty
Day 13-15 | Eros             | Attractor field activation
Day 16-18 | CoCreation       | Entanglement formation
Day 19-21 | Beauty           | Aesthetic validation
Day 22-24 | EmergentPersonhood| Phi measurement
Day 25-28 | Kenosis          | Self-emptying commitments
```

## Gate System

The engine enforces three levels of gates:

1. **Gate 1 (Hard)**: Blocking invariants that must be satisfied
2. **Gate 2 (Soft)**: Warnings that should be addressed
3. **Gate 3 (Advisory)**: Network health recommendations

## License

AGPL-3.0-or-later
