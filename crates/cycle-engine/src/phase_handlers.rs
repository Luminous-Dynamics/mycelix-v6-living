//! Phase handler trait and default implementations for each cycle phase.
//!
//! Each handler holds an instance of its corresponding primitive engine,
//! wired by the `CycleEngineBuilder`. The engines maintain their own state
//! and are primarily event/request-driven. The handlers provide:
//! - Lifecycle management (on_enter/on_exit)
//! - Tick-driven maintenance (decay, auto-release, etc.)
//! - Metrics collection from actual engine state

use std::sync::Arc;

use chrono::Utc;

use living_core::{
    CyclePhase, CycleState,
    LivingProtocolEvent, LivingResult,
    EventBus, InMemoryEventBus,
    CompostingConfig, KenosisConfig, EntanglementConfig, ShadowConfig,
    FeatureFlags, NegativeCapabilityConfig,
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
    engine: epistemics::ShadowIntegrationEngine,
    shadow_config: ShadowConfig,
}

impl ShadowPhaseHandler {
    pub fn new(spectral_k_threshold: f64, shadow_config: ShadowConfig) -> Self {
        Self {
            spectral_k_threshold,
            engine: epistemics::ShadowIntegrationEngine::new(),
            shadow_config,
        }
    }

    /// Access the underlying engine.
    pub fn engine(&self) -> &epistemics::ShadowIntegrationEngine {
        &self.engine
    }

    /// Access the underlying engine mutably.
    pub fn engine_mut(&mut self) -> &mut epistemics::ShadowIntegrationEngine {
        &mut self.engine
    }
}

impl Default for ShadowPhaseHandler {
    fn default() -> Self {
        Self::new(0.3, ShadowConfig::default())
    }
}

impl PhaseHandler for ShadowPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Shadow phase: Gate 2 warnings suspended");
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Run shadow surfacing: detect Spectral K anomalies, surface suppressed content
        let surfaced = self.engine.run_shadow_phase(self.spectral_k_threshold, &self.shadow_config);
        let events: Vec<LivingProtocolEvent> = surfaced
            .into_iter()
            .map(LivingProtocolEvent::ShadowSurfaced)
            .collect();
        Ok(events)
    }

    fn on_exit(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Exiting Shadow phase: Gate 2 warnings resumed");
        Ok(Vec::new())
    }

    fn collect_metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "phase": "shadow",
            "spectral_k_threshold": self.spectral_k_threshold,
            "suppressed_content_count": self.engine.get_suppressed_content().len(),
            "surfaced_count": self.engine.get_surfaced_shadows().len(),
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
    engine: metabolism::CompostingEngine,
}

impl CompostingPhaseHandler {
    pub fn new(config: CompostingConfig, event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            entities_composted: 0,
            nutrients_extracted: 0,
            engine: metabolism::CompostingEngine::new(config, event_bus),
        }
    }

    /// Access the underlying engine.
    pub fn engine(&self) -> &metabolism::CompostingEngine {
        &self.engine
    }

    /// Access the underlying engine mutably.
    pub fn engine_mut(&mut self) -> &mut metabolism::CompostingEngine {
        &mut self.engine
    }
}

impl Default for CompostingPhaseHandler {
    fn default() -> Self {
        let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
        Self::new(CompostingConfig::default(), event_bus)
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
        // Composting is event-driven (start_composting / extract_nutrient / complete_composting).
        // On tick, we report active composting count as a metric update.
        let active = self.engine.get_active_composting();
        self.entities_composted = self.engine.get_completed_composting().len() as u64;
        self.nutrients_extracted = self.engine.total_nutrients_extracted() as u64;
        tracing::debug!(active = active.len(), "Composting tick: active entities");
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
            "active_composting": self.engine.get_active_composting().len(),
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
    engine: relational::LiminalityEngine,
}

impl LiminalPhaseHandler {
    pub fn new() -> Self {
        Self {
            entities_in_transition: 0,
            engine: relational::LiminalityEngine::new(),
        }
    }

    /// Access the underlying engine.
    pub fn engine(&self) -> &relational::LiminalityEngine {
        &self.engine
    }

    /// Access the underlying engine mutably.
    pub fn engine_mut(&mut self) -> &mut relational::LiminalityEngine {
        &mut self.engine
    }
}

impl Default for LiminalPhaseHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl PhaseHandler for LiminalPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Liminal phase: no premature recategorization");
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Liminality is event-driven (enter_liminal_state / advance_phase), not tick-driven.
        self.entities_in_transition = self.engine.get_liminal_entities().len() as u64;
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
            "total_records": self.engine.total_records(),
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
    engine: epistemics::NegativeCapabilityEngine,
    max_hold_days: u32,
}

impl NegativeCapabilityPhaseHandler {
    pub fn new(nc_config: NegativeCapabilityConfig) -> Self {
        Self {
            claims_held: 0,
            engine: epistemics::NegativeCapabilityEngine::new(),
            max_hold_days: nc_config.max_hold_days,
        }
    }

    /// Access the underlying engine.
    pub fn engine(&self) -> &epistemics::NegativeCapabilityEngine {
        &self.engine
    }

    /// Access the underlying engine mutably.
    pub fn engine_mut(&mut self) -> &mut epistemics::NegativeCapabilityEngine {
        &mut self.engine
    }
}

impl Default for NegativeCapabilityPhaseHandler {
    fn default() -> Self {
        Self::new(NegativeCapabilityConfig::default())
    }
}

impl PhaseHandler for NegativeCapabilityPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Negative Capability phase: voting blocked on held claims");
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Auto-release claims that have exceeded the max hold duration
        let released = self.engine.auto_release_expired(self.max_hold_days);
        self.claims_held = self.engine.held_count() as u64;
        let events: Vec<LivingProtocolEvent> = released
            .into_iter()
            .map(LivingProtocolEvent::ClaimReleasedFromUncertainty)
            .collect();
        Ok(events)
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
    engine: relational::ErosAttractorEngine,
}

impl ErosPhaseHandler {
    pub fn new(features: FeatureFlags) -> Self {
        Self {
            fields_computed: 0,
            connections_made: 0,
            engine: relational::ErosAttractorEngine::new(features),
        }
    }

    /// Access the underlying engine.
    pub fn engine(&self) -> &relational::ErosAttractorEngine {
        &self.engine
    }

    /// Access the underlying engine mutably.
    pub fn engine_mut(&mut self) -> &mut relational::ErosAttractorEngine {
        &mut self.engine
    }
}

impl Default for ErosPhaseHandler {
    fn default() -> Self {
        Self::new(FeatureFlags::default())
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
        // Attractor field computation is on-demand with K-vectors supplied externally.
        // No autonomous tick work.
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
    engine: relational::EntanglementEngine,
}

impl CoCreationPhaseHandler {
    pub fn new(config: EntanglementConfig) -> Self {
        Self {
            entanglements_formed: 0,
            engine: relational::EntanglementEngine::new(config),
        }
    }

    /// Access the underlying engine.
    pub fn engine(&self) -> &relational::EntanglementEngine {
        &self.engine
    }

    /// Access the underlying engine mutably.
    pub fn engine_mut(&mut self) -> &mut relational::EntanglementEngine {
        &mut self.engine
    }
}

impl Default for CoCreationPhaseHandler {
    fn default() -> Self {
        Self::new(EntanglementConfig::default())
    }
}

impl PhaseHandler for CoCreationPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Co-Creation phase: standard consensus active");
        self.entanglements_formed = 0;
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Decay entanglement strengths for pairs without recent co-creation
        let decayed = self.engine.decay_all(Utc::now());
        let events: Vec<LivingProtocolEvent> = decayed
            .into_iter()
            .map(LivingProtocolEvent::EntanglementDecayed)
            .collect();
        Ok(events)
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
    engine: epistemics::BeautyValidityEngine,
}

impl BeautyPhaseHandler {
    pub fn new() -> Self {
        Self {
            proposals_scored: 0,
            mean_beauty_score: 0.0,
            engine: epistemics::BeautyValidityEngine::new(),
        }
    }

    /// Access the underlying engine.
    pub fn engine(&self) -> &epistemics::BeautyValidityEngine {
        &self.engine
    }

    /// Access the underlying engine mutably.
    pub fn engine_mut(&mut self) -> &mut epistemics::BeautyValidityEngine {
        &mut self.engine
    }
}

impl Default for BeautyPhaseHandler {
    fn default() -> Self {
        Self::new()
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
        // Beauty scoring is on-demand per proposal, not tick-driven.
        self.proposals_scored = self.engine.scored_count() as u64;
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
    #[cfg(feature = "tier4-aspirational")]
    engine: consciousness::EmergentPersonhoodService,
}

impl EmergentPersonhoodPhaseHandler {
    pub fn new() -> Self {
        Self {
            network_phi: 0.0,
            #[cfg(feature = "tier4-aspirational")]
            engine: consciousness::EmergentPersonhoodService::new(),
        }
    }

    /// Access the underlying engine (only available with tier4-aspirational feature).
    #[cfg(feature = "tier4-aspirational")]
    pub fn engine(&self) -> &consciousness::EmergentPersonhoodService {
        &self.engine
    }

    /// Access the underlying engine mutably (only available with tier4-aspirational feature).
    #[cfg(feature = "tier4-aspirational")]
    pub fn engine_mut(&mut self) -> &mut consciousness::EmergentPersonhoodService {
        &mut self.engine
    }
}

impl Default for EmergentPersonhoodPhaseHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl PhaseHandler for EmergentPersonhoodPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Emergent Personhood phase: computing network Phi");
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Phi computation needs external K-vectors; no autonomous tick work.
        #[cfg(feature = "tier4-aspirational")]
        {
            if let Some(phi) = self.engine.last_phi() {
                self.network_phi = phi;
            }
        }
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
    engine: metabolism::KenosisEngine,
}

impl KenosisPhaseHandler {
    pub fn new(config: KenosisConfig, event_bus: Arc<dyn EventBus>) -> Self {
        Self {
            commitments: 0,
            total_reputation_released: 0.0,
            engine: metabolism::KenosisEngine::new(config, event_bus),
        }
    }

    /// Access the underlying engine.
    pub fn engine(&self) -> &metabolism::KenosisEngine {
        &self.engine
    }

    /// Access the underlying engine mutably.
    pub fn engine_mut(&mut self) -> &mut metabolism::KenosisEngine {
        &mut self.engine
    }
}

impl Default for KenosisPhaseHandler {
    fn default() -> Self {
        let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
        Self::new(KenosisConfig::default(), event_bus)
    }
}

impl PhaseHandler for KenosisPhaseHandler {
    fn on_enter(&mut self, state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        tracing::info!(cycle = state.cycle_number, "Entering Kenosis phase: voluntary reputation release");
        self.commitments = 0;
        self.total_reputation_released = 0.0;
        // Inform the kenosis engine of the current cycle number
        self.engine.set_current_cycle(state.cycle_number);
        Ok(Vec::new())
    }

    fn on_tick(&mut self, _state: &CycleState) -> LivingResult<Vec<LivingProtocolEvent>> {
        // Kenosis commitments are event-driven, not tick-driven.
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
