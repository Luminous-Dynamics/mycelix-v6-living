//! # Metabolism Engine
//!
//! Module A of the Mycelix v6.0 Living Protocol Layer.
//! Implements primitives [1]-[4]:
//!
//! - **[1] Composting**: Decomposition of failed entities, nutrient extraction for the DKG.
//! - **[2] Wound Healing**: 4-phase FSM replacing punitive slashing with restorative healing.
//! - **[3] Metabolic Trust**: Throughput-based trust scoring extending MATL.
//! - **[4] Kenosis**: Self-emptying mechanism for voluntary reputation release.
//!
//! ## Constitutional Alignment
//!
//! - Composting and Wound Healing align with **Sacred Reciprocity** (Harmony 6):
//!   failures are recycled, not discarded; harm is healed, not punished.
//! - Kenosis aligns with **Evolutionary Progression** (Harmony 7):
//!   voluntary self-limitation enables collective growth.
//!
//! ## Three Gates Compliance
//!
//! Each primitive enforces Gate 1 (mathematical invariants) and Gate 2 (constitutional
//! warnings). Gate 3 (social consequences) is handled by the event bus and downstream
//! subscribers.

pub mod composting;
pub mod kenosis;
pub mod metabolic_trust;
pub mod wound_healing;

pub use composting::CompostingEngine;
pub use kenosis::KenosisEngine;
pub use metabolic_trust::MetabolicTrustEngine;
pub use wound_healing::WoundHealingEngine;
