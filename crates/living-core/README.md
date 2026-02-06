# living-core

Shared types and events for Mycelix v6.0 Living Protocol Layer.

## Overview

This crate provides the foundational types, events, and error handling for the Living Protocol Layer:

- **Types**: Core data structures (Agent, Entity, Cycle state)
- **Events**: Event system for inter-component communication
- **Errors**: Unified error handling across all crates
- **Validation**: Shared validation logic

## Installation

```toml
[dependencies]
living-core = "0.6"
```

## Usage

```rust
use living_core::{AgentId, CyclePhase, LivingEvent};

// Create an agent ID
let agent = AgentId::new();

// Check cycle phase
let phase = CyclePhase::Shadow;
assert!(phase.permits_operation("reflection"));
```

## Features

- `serde` - Serialization/deserialization support (enabled by default)
- `validation` - Additional validation utilities

## License

AGPL-3.0-or-later
