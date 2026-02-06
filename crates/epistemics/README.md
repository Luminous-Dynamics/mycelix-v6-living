# epistemics

Epistemic Deepening primitives for Mycelix Living Protocol.

## Overview

This crate implements ways of knowing that go beyond conventional rationality:

- **Shadow Integration**: Surface and integrate suppressed content
- **Negative Capability**: Hold uncertainty without irritable reaching
- **Silence as Signal**: Meaningful absence in communication
- **Beauty as Validity**: Aesthetic dimension of truth

## Installation

```toml
[dependencies]
epistemics = "0.6"
```

## Usage

```rust
use epistemics::{ShadowIntegration, NegativeCapability, BeautyValidator};

// Shadow work during Shadow phase
let shadow = ShadowIntegration::new();
let suppressed = shadow.surface_content(agent)?;
shadow.integrate(agent, suppressed)?;

// Hold uncertainty
let capability = NegativeCapability::new();
capability.hold_uncertainty(proposal, duration)?;

// Validate through beauty
let validator = BeautyValidator::new();
let score = validator.assess_beauty(artifact)?;
```

## The Epistemic Cycle

During different phases, different epistemic modes are emphasized:

- **Shadow Phase (Day 1-3)**: Shadow Integration active
- **Liminal Phase (Day 7-9)**: Negative Capability emphasized
- **Beauty Phase (Day 19-21)**: Beauty as Validity for proposals

## License

AGPL-3.0-or-later
