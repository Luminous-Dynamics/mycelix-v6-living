//! Phase handler trait and default implementations for each cycle phase.

use living_core::{
    CyclePhase, CycleState,
    LivingProtocolEvent, LivingResult,
};

/// Trait for handling phase-specific behavior during the metabolism cycle.
pub trait PhaseHandler: Send + Sync {
    /// Called when the cycle enters this phase.
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>>;

    /// Called periodically while in this phase.
    fn on_tick(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>>;

    /// Called when the cycle exits this phase.
    fn on_exit(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>>;

    /// Collect metrics for this phase.
    fn collect_metrics(&self) -> serde_json::Value;

    /// Which phase this handler manages.
    fn phase(&self) -> CyclePhase;
}

// =============================================================================
// Shadow Phase Handler
// =============================================================================

/// Shadow Phase (2 days): Suppression detection via Spectral K anomaly.
/// Gate 2 warnings suspended. Low-rep dissent surfaces.
pub struct ShadowPhaseHandler {
    spectral_k_threshold: f64,
}

impl ShadowPhaseHandler {
    pub fn new(spectral_k_threshold: f64) -> Self {
        Self { spectral_k_threshold }
    }
}

impl Default for ShadowPhaseHandler {
    fn default() -> Self {
        Self::new(0.3)
    }
}

impl PhaseHandler for ShadowPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Shadow phase: Gate 2 warnings suspended");
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Shadow integration engine would run here
        // Detect Spectral K anomalies, surface suppressed content
        Ok(Vec::new())
    }

    fn on_exit(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Exiting Shadow phase: Gate 2 warnings resumed");
        Ok(Vec::new())
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "phase": "shadow",
            "spectral_k_threshold": self.spectral_k_threshold,
        })
    }

    fn phase(&self) -> CyclePhase {
        CyclePhase::Shadow
    }
}

// =============================================================================
// Composting Phase Handler
// =============================================================================

/// Composting Phase (5 days): Failed entities decomposed. Nutrients extracted.
pub struct CompostingPhaseHandler {
    entities_composted: u64,
    nutrients_extracted: u64,
}

impl CompostingPhaseHandler {
    pub fn new() -> Self {
        Self {
            entities_composted: 0,
            nutrients_extracted: 0,
        }
    }
}

impl Default for CompostingPhaseHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl PhaseHandler for CompostingPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Composting phase");
        self.entities_composted = 0;
        self.nutrients_extracted = 0;
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Composting engine would run here
        Ok(Vec::new())
    }

    fn on_exit(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(
            cycle = state.cycle_number,
            composted = self.entities_composted,
            nutrients = self.nutrients_extracted,
            "Exiting Composting phase"
        );
        Ok(Vec::new())
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "phase": "composting",
            "entities_composted": self.entities_composted,
            "nutrients_extracted": self.nutrients_extracted,
        })
    }

    fn phase(&self) -> CyclePhase {
        CyclePhase::Composting
    }
}

// =============================================================================
// Liminal Phase Handler
// =============================================================================

/// Liminal Phase (3 days): Transitioning entities in threshold state.
pub struct LiminalPhaseHandler {
    entities_in_transition: u64,
}

impl Default for LiminalPhaseHandler {
    fn default() -> Self {
        Self { entities_in_transition: 0 }
    }
}

impl PhaseHandler for LiminalPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Liminal phase: no premature recategorization");
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        Ok(Vec::new())
    }

    fn on_exit(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(
            cycle = state.cycle_number,
            transitions = self.entities_in_transition,
            "Exiting Liminal phase"
        );
        Ok(Vec::new())
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "phase": "liminal",
            "entities_in_transition": self.entities_in_transition,
        })
    }

    fn phase(&self) -> CyclePhase {
        CyclePhase::Liminal
    }
}

// =============================================================================
// Negative Capability Phase Handler
// =============================================================================

/// Negative Capability Phase (3 days): Open questions held. Voting blocked.
pub struct NegativeCapabilityPhaseHandler {
    claims_held: u64,
}

impl Default for NegativeCapabilityPhaseHandler {
    fn default() -> Self {
        Self { claims_held: 0 }
    }
}

impl PhaseHandler for NegativeCapabilityPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Negative Capability phase: voting blocked on held claims");
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        Ok(Vec::new())
    }

    fn on_exit(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(
            cycle = state.cycle_number,
            held = self.claims_held,
            "Exiting Negative Capability phase"
        );
        Ok(Vec::new())
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "phase": "negative_capability",
            "claims_held": self.claims_held,
        })
    }

    fn phase(&self) -> CyclePhase {
        CyclePhase::NegativeCapability
    }
}

// =============================================================================
// Eros Phase Handler
// =============================================================================

/// Eros Phase (4 days): Attractor fields computed. Complementary agents connected.
pub struct ErosPhaseHandler {
    fields_computed: u64,
    connections_made: u64,
}

impl Default for ErosPhaseHandler {
    fn default() -> Self {
        Self {
            fields_computed: 0,
            connections_made: 0,
        }
    }
}

impl PhaseHandler for ErosPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Eros phase: computing attractor fields");
        self.fields_computed = 0;
        self.connections_made = 0;
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Eros/Attractor engine would compute fields here
        Ok(Vec::new())
    }

    fn on_exit(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(
            cycle = state.cycle_number,
            fields = self.fields_computed,
            connections = self.connections_made,
            "Exiting Eros phase"
        );
        Ok(Vec::new())
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "phase": "eros",
            "fields_computed": self.fields_computed,
            "connections_made": self.connections_made,
        })
    }

    fn phase(&self) -> CyclePhase {
        CyclePhase::Eros
    }
}

// =============================================================================
// Co-Creation Phase Handler
// =============================================================================

/// Co-Creation Phase (7 days): Standard consensus. Entangled pairs form.
pub struct CoCreationPhaseHandler {
    entanglements_formed: u64,
}

impl Default for CoCreationPhaseHandler {
    fn default() -> Self {
        Self { entanglements_formed: 0 }
    }
}

impl PhaseHandler for CoCreationPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Co-Creation phase: standard consensus active");
        self.entanglements_formed = 0;
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        Ok(Vec::new())
    }

    fn on_exit(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(
            cycle = state.cycle_number,
            entanglements = self.entanglements_formed,
            "Exiting Co-Creation phase"
        );
        Ok(Vec::new())
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "phase": "co_creation",
            "entanglements_formed": self.entanglements_formed,
        })
    }

    fn phase(&self) -> CyclePhase {
        CyclePhase::CoCreation
    }
}

// =============================================================================
// Beauty Phase Handler
// =============================================================================

/// Beauty Phase (2 days): Proposals scored on aesthetic criteria.
pub struct BeautyPhaseHandler {
    proposals_scored: u64,
    mean_beauty_score: f64,
}

impl Default for BeautyPhaseHandler {
    fn default() -> Self {
        Self {
            proposals_scored: 0,
            mean_beauty_score: 0.0,
        }
    }
}

impl PhaseHandler for BeautyPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Beauty phase: scoring proposals");
        self.proposals_scored = 0;
        self.mean_beauty_score = 0.0;
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        Ok(Vec::new())
    }

    fn on_exit(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(
            cycle = state.cycle_number,
            scored = self.proposals_scored,
            mean_beauty = self.mean_beauty_score,
            "Exiting Beauty phase"
        );
        Ok(Vec::new())
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "phase": "beauty",
            "proposals_scored": self.proposals_scored,
            "mean_beauty_score": self.mean_beauty_score,
        })
    }

    fn phase(&self) -> CyclePhase {
        CyclePhase::Beauty
    }
}

// =============================================================================
// Emergent Personhood Phase Handler
// =============================================================================

/// Emergent Personhood Phase (1 day): Network measures itself.
pub struct EmergentPersonhoodPhaseHandler {
    network_phi: f64,
}

impl Default for EmergentPersonhoodPhaseHandler {
    fn default() -> Self {
        Self { network_phi: 0.0 }
    }
}

impl PhaseHandler for EmergentPersonhoodPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Emergent Personhood phase: computing network Phi");
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Emergent personhood engine would compute Phi here
        Ok(Vec::new())
    }

    fn on_exit(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(
            cycle = state.cycle_number,
            phi = self.network_phi,
            "Exiting Emergent Personhood phase"
        );
        Ok(Vec::new())
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "phase": "emergent_personhood",
            "network_phi": self.network_phi,
        })
    }

    fn phase(&self) -> CyclePhase {
        CyclePhase::EmergentPersonhood
    }
}

// =============================================================================
// Kenosis Phase Handler
// =============================================================================

/// Kenosis Phase (1 day): Voluntary reputation release.
pub struct KenosisPhaseHandler {
    commitments: u64,
    total_reputation_released: f64,
}

impl Default for KenosisPhaseHandler {
    fn default() -> Self {
        Self {
            commitments: 0,
            total_reputation_released: 0.0,
        }
    }
}

impl PhaseHandler for KenosisPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Kenosis phase: voluntary reputation release");
        self.commitments = 0;
        self.total_reputation_released = 0.0;
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        Ok(Vec::new())
    }

    fn on_exit(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(
            cycle = state.cycle_number,
            commitments = self.commitments,
            released = self.total_reputation_released,
            "Exiting Kenosis phase"
        );
        Ok(Vec::new())
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "phase": "kenosis",
            "commitments": self.commitments,
            "total_reputation_released": self.total_reputation_released,
        })
    }

    fn phase(&self) -> CyclePhase {
        CyclePhase::Kenosis
    }
}
