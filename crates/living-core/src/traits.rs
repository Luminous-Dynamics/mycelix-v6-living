//! Traits that all Living Protocol primitives must implement.

use crate::error::LivingResult;
use crate::events::LivingProtocolEvent;
use crate::types::{CyclePhase, Gate1Check, Gate2Warning};

/// Every primitive must implement this trait to integrate with the Metabolism Cycle.
pub trait LivingPrimitive: Send + Sync {
    /// Unique identifier for this primitive (e.g., "composting", "wound_healing").
    fn primitive_id(&self) -> &str;

    /// Numeric ID [1-21].
    fn primitive_number(&self) -> u8;

    /// Module this primitive belongs to.
    fn module(&self) -> PrimitiveModule;

    /// Implementation tier (1-4).
    fn tier(&self) -> u8;

    /// Called when the Metabolism Cycle enters a new phase.
    /// Primitives can activate/deactivate based on the phase.
    fn on_phase_change(&mut self, new_phase: CyclePhase) -> LivingResult<Vec<LivingProtocolEvent>>;

    /// Gate 1 check: mathematical invariants.
    /// Returns list of checks; all must pass.
    fn gate1_check(&self) -> Vec<Gate1Check>;

    /// Gate 2 check: constitutional warnings.
    /// Returns list of warnings (non-blocking).
    fn gate2_check(&self) -> Vec<Gate2Warning>;

    /// Whether this primitive is active in the given phase.
    fn is_active_in_phase(&self, phase: CyclePhase) -> bool;

    /// Collect metrics for phase transition.
    fn collect_metrics(&self) -> serde_json::Value;
}

/// Module classification for primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveModule {
    /// Module A: Metabolism Engine [1-4]
    Metabolism,
    /// Module B: Consciousness Field [5-8]
    Consciousness,
    /// Module C: Epistemic Deepening [9-12]
    Epistemics,
    /// Module D: Relational Field [13-16]
    Relational,
    /// Module E: Structural Emergence [17-21]
    Structural,
}

/// Trait for primitives that manage agent reputation.
pub trait ReputationAware {
    /// Get current reputation for an agent.
    fn get_reputation(&self, agent_did: &str) -> LivingResult<f64>;

    /// Apply reputation delta (positive or negative).
    fn apply_reputation_delta(&mut self, agent_did: &str, delta: f64) -> LivingResult<f64>;
}

/// Trait for primitives that interact with the DKG.
pub trait DkgAware {
    /// Publish a nutrient/learning to the DKG.
    fn publish_to_dkg(
        &self,
        content: &str,
        classification: crate::types::EpistemicClassification,
    ) -> LivingResult<String>;

    /// Query the DKG for related content.
    fn query_dkg(&self, query: &str) -> LivingResult<Vec<String>>;
}

/// Trait for primitives that need access to the Spectral K metric.
pub trait SpectralKAware {
    /// Get current spectral gap (λ₂).
    fn spectral_k(&self) -> f64;

    /// Whether the network is fragmented (λ₂ < threshold).
    fn is_fragmented(&self, threshold: f64) -> bool {
        self.spectral_k() < threshold
    }
}

/// Trait for primitives that participate in Three Gates enforcement.
pub trait ThreeGatesCompliant {
    /// Gate 1: Absolute mathematical invariants.
    fn enforce_gate1(&self) -> LivingResult<()>;

    /// Gate 2: Constitutional warnings with reputation impact.
    fn evaluate_gate2(&self) -> Vec<Gate2Warning>;

    /// Gate 3: Social consequence signals.
    fn emit_gate3_signals(&self) -> Vec<crate::types::Gate3Consequence>;
}
