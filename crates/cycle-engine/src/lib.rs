//! # Metabolism Cycle Engine
//!
//! The central orchestrator for the Mycelix v6.0 Living Protocol Layer.
//! Manages the 28-day (lunar) metabolism cycle that connects all 21 primitives
//! into a coherent lifecycle.
//!
//! ## Cycle Phases (28 days total)
//!
//! ```text
//! Shadow (2d) → Composting (5d) → Liminal (3d) → Negative Capability (3d) →
//! Eros (4d) → Co-Creation (7d) → Beauty (2d) → Emergent Personhood (1d) →
//! Kenosis (1d) → [back to Shadow]
//! ```

pub mod engine;
pub mod phase_handlers;
pub mod scheduler;

pub use engine::*;
pub use phase_handlers::*;
pub use scheduler::*;
