//! # Epistemics — Module C: Epistemic Deepening
//!
//! Implements Living Protocol primitives [9]-[12]:
//!
//! - **[9] Shadow Integration** — Periodic surfacing of suppressed dissent.
//!   Triggers on Spectral K anomaly (low lambda-2 suggests groupthink).
//!   NEVER surfaces Gate 1-protected content.
//!
//! - **[10] Negative Capability** — `HeldInUncertainty` claim status with
//!   voting blocked. The simplest primitive: holds open questions open.
//!
//! - **[11] Silence as Signal** — Meaningful absence detection backed by
//!   `PresenceProof`. Distinguishes deliberate withholding, contemplation,
//!   and dissent-through-absence.
//!
//! - **[12] Beauty as Validity** — Scoring proposals on symmetry, economy,
//!   resonance, surprise, and completeness. Used during the Beauty phase
//!   of the 28-day Metabolism Cycle.
//!
//! All four primitives implement the `LivingPrimitive` trait from `living-core`
//! and emit events consumed by the Metabolism Cycle orchestrator.

pub mod beauty_validity;
pub mod negative_capability;
pub mod shadow_integration;
pub mod silence_signal;

#[cfg(test)]
mod proptest_beauty;

pub use beauty_validity::*;
pub use negative_capability::*;
pub use shadow_integration::*;
pub use silence_signal::*;
