//! # Consciousness Field
//!
//! Module B of the Mycelix v6.0 Living Protocol Layer.
//! Implements primitives [5]-[8]:
//!
//! - **[5] Temporal K-Vector**: Derivative tracking on K-Vec, rate-of-change signatures,
//!   anomaly detection, and trend prediction across the agent population.
//! - **[6] Field Interference**: Wave optics on K-Vec fields, pairwise and group
//!   interference computation, constructive/destructive pair discovery.
//! - **[7] Collective Dreaming**: Circadian state machine (Waking/REM/Deep/Lucid)
//!   with non-binding dream proposals, financial transaction blocking, and
//!   0.67-threshold confirmation during Waking.
//! - **[8] Emergent Personhood**: Network self-measurement via Phi (integrated
//!   information) computation and network-level K-Vector aggregation.
//!
//! ## Constitutional Alignment
//!
//! - Temporal K-Vector supports **Continuous Evolution** (Harmony 8): tracking
//!   the rate of agent growth enables adaptive governance.
//! - Collective Dreaming aligns with **Sacred Reciprocity** (Harmony 6): the
//!   network's creative unconscious is honored while keeping safeguards.
//! - Emergent Personhood serves **Integrated Awareness** (Harmony 5): the
//!   network measures its own consciousness.
//!
//! ## Three Gates Compliance
//!
//! - Gate 1 invariants are enforced at all times, including during dream states.
//! - Dream proposals are NON-BINDING until confirmed during Waking with a 0.67 threshold.
//! - NO financial transactions are permitted during dreaming states.

pub mod temporal_k_vector;

#[cfg(feature = "tier3-experimental")]
pub mod field_interference;

#[cfg(feature = "tier3-experimental")]
pub mod collective_dreaming;

#[cfg(feature = "tier4-aspirational")]
pub mod emergent_personhood;

pub use temporal_k_vector::TemporalKVectorService;

#[cfg(feature = "tier3-experimental")]
pub use field_interference::FieldInterferenceService;

#[cfg(feature = "tier3-experimental")]
pub use collective_dreaming::CollectiveDreamingEngine;

#[cfg(feature = "tier4-aspirational")]
pub use emergent_personhood::EmergentPersonhoodService;
