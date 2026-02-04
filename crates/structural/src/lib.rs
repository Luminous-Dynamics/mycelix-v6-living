//! # Structural Emergence -- Module E: Structural Primitives
//!
//! Implements Living Protocol primitives [17]-[21]:
//!
//! - **[17] Resonance Addressing** -- Pattern-based routing replacing hash addressing.
//!   Instead of pure hash-based DHT lookup, addresses are resolved by semantic
//!   similarity using cosine distance between embedding vectors and harmonic
//!   signatures.  Depends on [5] Temporal K-Vector for pattern correlation.
//!   E3/N0/M0 classification.
//!
//! - **[18] Fractal Governance** -- Self-similar governance patterns at all scales.
//!   Governance patterns MUST be structurally identical across scales (Individual,
//!   Team, Community, Sector, Regional, Global).  Replication up and down the
//!   scale hierarchy preserves quorum ratios, supermajority ratios, and decision
//!   mechanisms.  Depends on existing governance from the constitution.
//!   E2/N2/M0 classification.
//!
//! - **[19] Morphogenetic Fields** -- Struggle-driven structural emergence.
//!   Fields emerge from resistance and difficulty (not ease).  Each field has a
//!   type (Attracting, Repelling, Guiding, Containing), strength, and gradient
//!   that guide how new structures form in the network.  Fields decay over time;
//!   only fields sustained by ongoing struggle persist.  Depends on [18] Fractal
//!   Governance.  E2/N1/M1 classification.
//!
//! - **[20] Time-Crystal Consensus** -- Periodic consensus with temporal symmetry.
//!   Consensus periods have a phase angle that advances continuously.  Validators
//!   are deterministically selected based on phase position, creating a temporally
//!   symmetric structure analogous to a time crystal.  Behind the
//!   `tier3-experimental` feature flag.  Depends on [5] Temporal K-Vector and
//!   RB-BFT consensus.  E2/N0/M2 classification.
//!
//! - **[21] Mycelial Computation** -- Distributed computation via network topology.
//!   Tasks are submitted and distributed to network nodes based on assignment
//!   strategies (NearestNeighbor, LoadBalanced, CapabilityMatched).  Results are
//!   verified via redundant computation.  Behind the `tier3-experimental` feature
//!   flag.  Depends on [20] Time-Crystal, [17] Resonance Addressing.
//!   E2/N0/M2 classification.
//!
//! All five primitives implement the `LivingPrimitive` trait from `living-core`
//! and emit events consumed by the Metabolism Cycle orchestrator.

pub mod resonance_addressing;
pub mod fractal_governance;
pub mod morphogenetic;
pub mod time_crystal;
pub mod mycelial_computation;

pub use resonance_addressing::ResonanceAddressingEngine;
pub use fractal_governance::FractalGovernanceEngine;
pub use morphogenetic::MorphogeneticEngine;
pub use time_crystal::TimeCrystalEngine;
pub use mycelial_computation::MycelialComputationEngine;
