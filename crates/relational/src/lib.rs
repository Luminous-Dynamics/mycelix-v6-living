//! # Relational Field -- Module D: Relational Primitives
//!
//! Implements Living Protocol primitives [13]-[16]:
//!
//! - **[13] Entangled Pairs** -- Correlation detection between agents.
//!   Entanglement forms when two agents accumulate sufficient co-creation history
//!   and DECAYS without continued interaction. Depends on [5] Temporal K-Vector
//!   for correlation detection.  E3/N0/M1 classification.
//!
//! - **[14] Eros / Attractor Fields** -- Field computation for *complementary*
//!   agents (not similar -- agents whose strengths fill each other's gaps).
//!   Behind the `tier3-experimental` feature flag.  Depends on [5] Temporal
//!   K-Vector and [6] Field Interference.  E1/N1/M2 classification.
//!
//! - **[15] Liminality** -- State machine for identity transitions.  Phase
//!   progression is strictly forward-only: PreLiminal -> Liminal -> PostLiminal
//!   -> Integrated.  Entities in liminal state CANNOT be prematurely
//!   recategorized.  Active during the Liminal phase of the 28-day Metabolism
//!   Cycle.  Constitutional: preserves Right to Fair Recourse.  E2/N2/M1.
//!
//! - **[16] Inter-Species Participation** -- Cross-species protocol bridge for
//!   Human, AI, DAO, Sensor, and Ecological participants.  Behind the
//!   `tier4-aspirational` feature flag.  Depends on [17] Resonance Addressing
//!   and [11] Silence as Signal.  E1/N2/M2 classification.
//!
//! All four primitives implement the `LivingPrimitive` trait from `living-core`
//! and emit events consumed by the Metabolism Cycle orchestrator.

pub mod entangled_pairs;
pub mod eros_attractor;
pub mod inter_species;
pub mod liminality;

pub use entangled_pairs::*;
pub use eros_attractor::*;
pub use inter_species::*;
pub use liminality::*;
