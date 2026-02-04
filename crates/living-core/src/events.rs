//! Events emitted by all Living Protocol primitives.
//! The Metabolism Cycle orchestrator listens to these events to coordinate behavior.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::types::*;

/// Top-level event enum for all Living Protocol events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LivingProtocolEvent {
    // =========================================================================
    // Module A: Metabolism Engine
    // =========================================================================
    /// [1] Composting: entity decomposition started
    CompostingStarted(CompostingStartedEvent),
    /// [1] Composting: nutrient extracted
    NutrientExtracted(NutrientExtractedEvent),
    /// [1] Composting: decomposition complete
    CompostingCompleted(CompostingCompletedEvent),

    /// [2] Wound Healing: wound created
    WoundCreated(WoundCreatedEvent),
    /// [2] Wound Healing: phase advanced
    WoundPhaseAdvanced(WoundPhaseAdvancedEvent),
    /// [2] Wound Healing: restitution fulfilled
    RestitutionFulfilled(RestitutionFulfilledEvent),
    /// [2] Wound Healing: scar tissue formed
    ScarTissueFormed(ScarTissueFormedEvent),

    /// [3] Metabolic Trust: score updated
    MetabolicTrustUpdated(MetabolicTrustUpdatedEvent),

    /// [4] Kenosis: reputation release committed
    KenosisCommitted(KenosisCommittedEvent),
    /// [4] Kenosis: reputation actually released
    KenosisExecuted(KenosisExecutedEvent),

    // =========================================================================
    // Module B: Consciousness Field
    // =========================================================================
    /// [5] Temporal K-Vector: derivative computed
    TemporalKVectorUpdated(TemporalKVectorUpdatedEvent),

    /// [6] Field Interference: interference pattern detected
    FieldInterferenceDetected(FieldInterferenceDetectedEvent),

    /// [7] Collective Dreaming: dream state changed
    DreamStateChanged(DreamStateChangedEvent),
    /// [7] Collective Dreaming: dream proposal generated
    DreamProposalGenerated(DreamProposalGeneratedEvent),

    /// [8] Emergent Personhood: network Phi computed
    NetworkPhiComputed(NetworkPhiComputedEvent),

    // =========================================================================
    // Module C: Epistemic Deepening
    // =========================================================================
    /// [9] Shadow Integration: suppressed content surfaced
    ShadowSurfaced(ShadowSurfacedEvent),

    /// [10] Negative Capability: claim held in uncertainty
    ClaimHeldInUncertainty(ClaimHeldEvent),
    /// [10] Negative Capability: claim released from uncertainty
    ClaimReleasedFromUncertainty(ClaimReleasedEvent),

    /// [11] Silence as Signal: meaningful silence detected
    SilenceDetected(SilenceDetectedEvent),

    /// [12] Beauty as Validity: proposal scored
    BeautyScored(BeautyScoredEvent),

    // =========================================================================
    // Module D: Relational Field
    // =========================================================================
    /// [13] Entangled Pairs: entanglement formed
    EntanglementFormed(EntanglementFormedEvent),
    /// [13] Entangled Pairs: entanglement decayed to zero
    EntanglementDecayed(EntanglementDecayedEvent),

    /// [14] Eros / Attractor: attractor field computed
    AttractorFieldComputed(AttractorFieldComputedEvent),

    /// [15] Liminality: entity entered liminal state
    LiminalTransitionStarted(LiminalTransitionStartedEvent),
    /// [15] Liminality: entity emerged from liminal state
    LiminalTransitionCompleted(LiminalTransitionCompletedEvent),

    /// [16] Inter-Species: new species participant registered
    InterSpeciesRegistered(InterSpeciesRegisteredEvent),

    // =========================================================================
    // Module E: Structural Emergence
    // =========================================================================
    /// [17] Resonance Addressing: new resonance address created
    ResonanceAddressCreated(ResonanceAddressCreatedEvent),

    /// [18] Fractal Governance: governance pattern replicated
    FractalPatternReplicated(FractalPatternReplicatedEvent),

    /// [19] Morphogenetic Fields: field strength changed
    MorphogeneticFieldUpdated(MorphogeneticFieldUpdatedEvent),

    /// [20] Time-Crystal: new consensus period started
    TimeCrystalPeriodStarted(TimeCrystalPeriodStartedEvent),

    /// [21] Mycelial Computation: task distributed
    MycelialTaskDistributed(MycelialTaskDistributedEvent),
    /// [21] Mycelial Computation: task completed
    MycelialTaskCompleted(MycelialTaskCompletedEvent),

    // =========================================================================
    // Metabolism Cycle
    // =========================================================================
    /// Cycle phase transitioned
    PhaseTransitioned(PhaseTransitionedEvent),
    /// New cycle started
    CycleStarted(CycleStartedEvent),
}

// =============================================================================
// Module A: Metabolism Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompostingStartedEvent {
    pub record_id: String,
    pub entity_type: CompostableEntity,
    pub entity_id: EntityId,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NutrientExtractedEvent {
    pub record_id: String,
    pub nutrient: Nutrient,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompostingCompletedEvent {
    pub record_id: String,
    pub entity_id: EntityId,
    pub total_nutrients: usize,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WoundCreatedEvent {
    pub wound_id: String,
    pub agent_did: Did,
    pub severity: WoundSeverity,
    pub cause: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WoundPhaseAdvancedEvent {
    pub wound_id: String,
    pub agent_did: Did,
    pub from: WoundPhase,
    pub to: WoundPhase,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestitutionFulfilledEvent {
    pub wound_id: String,
    pub agent_did: Did,
    pub restitution: RestitutionRequirement,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScarTissueFormedEvent {
    pub wound_id: String,
    pub agent_did: Did,
    pub scar: ScarTissue,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetabolicTrustUpdatedEvent {
    pub agent_did: Did,
    pub old_score: f64,
    pub new_score: f64,
    pub components: MetabolicTrustScore,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KenosisCommittedEvent {
    pub commitment_id: String,
    pub agent_did: Did,
    pub release_percentage: f64,
    pub reputation_released: f64,
    pub cycle_number: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KenosisExecutedEvent {
    pub commitment_id: String,
    pub agent_did: Did,
    pub reputation_before: f64,
    pub reputation_after: f64,
    pub timestamp: DateTime<Utc>,
}

// =============================================================================
// Module B: Consciousness Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalKVectorUpdatedEvent {
    pub agent_did: Did,
    pub derivatives: Vec<f64>,
    pub rate_of_change: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInterferenceDetectedEvent {
    pub agents: Vec<Did>,
    pub interference_type: InterferenceType,
    pub amplitude: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterferenceType {
    Constructive,
    Destructive,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamStateChangedEvent {
    pub from: DreamState,
    pub to: DreamState,
    pub network_participation: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamProposalGeneratedEvent {
    pub proposal: DreamProposal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPhiComputedEvent {
    pub phi: f64,
    pub node_count: u64,
    pub integration_score: f64,
    pub timestamp: DateTime<Utc>,
}

// =============================================================================
// Module C: Epistemic Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowSurfacedEvent {
    pub shadow: ShadowRecord,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimHeldEvent {
    pub claim_id: ClaimId,
    pub reason: String,
    pub earliest_resolution: DateTime<Utc>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimReleasedEvent {
    pub claim_id: ClaimId,
    pub resolution: String,
    pub held_duration: chrono::Duration,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilenceDetectedEvent {
    pub agent_did: Did,
    pub topic: String,
    pub classification: SilenceClassification,
    pub duration: chrono::Duration,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeautyScoredEvent {
    pub proposal_id: String,
    pub score: BeautyScore,
    pub scorer_did: Did,
    pub timestamp: DateTime<Utc>,
}

// =============================================================================
// Module D: Relational Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntanglementFormedEvent {
    pub pair: EntangledPair,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntanglementDecayedEvent {
    pub pair_id: String,
    pub agent_a: Did,
    pub agent_b: Did,
    pub final_strength: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttractorFieldComputedEvent {
    pub field_center: Did,
    pub attracted_agents: Vec<(Did, f64)>,
    pub field_strength: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiminalTransitionStartedEvent {
    pub record: LiminalRecord,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiminalTransitionCompletedEvent {
    pub record_id: String,
    pub entity_did: Did,
    pub new_identity: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterSpeciesRegisteredEvent {
    pub participant: InterSpeciesParticipant,
    pub timestamp: DateTime<Utc>,
}

// =============================================================================
// Module E: Structural Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceAddressCreatedEvent {
    pub address: ResonanceAddress,
    pub owner_did: Did,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FractalPatternReplicatedEvent {
    pub pattern: FractalGovernancePattern,
    pub parent_scale: GovernanceScale,
    pub child_scale: GovernanceScale,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphogeneticFieldUpdatedEvent {
    pub field: MorphogeneticField,
    pub old_strength: f64,
    pub new_strength: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeCrystalPeriodStartedEvent {
    pub period: TimeCrystalPeriod,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MycelialTaskDistributedEvent {
    pub task: MycelialTask,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MycelialTaskCompletedEvent {
    pub task_id: String,
    pub result_hash: HashDigest,
    pub duration: chrono::Duration,
    pub timestamp: DateTime<Utc>,
}

// =============================================================================
// Cycle Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseTransitionedEvent {
    pub transition: PhaseTransition,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleStartedEvent {
    pub cycle_number: u64,
    pub started_at: DateTime<Utc>,
}

// =============================================================================
// Event Bus Trait
// =============================================================================

/// Trait for publishing and subscribing to Living Protocol events.
pub trait EventBus: Send + Sync {
    /// Publish an event to all subscribers.
    fn publish(&self, event: LivingProtocolEvent);

    /// Subscribe to events of a specific type.
    fn subscribe(&self, handler: Box<dyn Fn(&LivingProtocolEvent) + Send + Sync>);
}

/// Simple in-memory event bus for testing.
pub struct InMemoryEventBus {
    handlers: std::sync::Mutex<Vec<Box<dyn Fn(&LivingProtocolEvent) + Send + Sync>>>,
    history: std::sync::Mutex<Vec<LivingProtocolEvent>>,
}

impl InMemoryEventBus {
    pub fn new() -> Self {
        Self {
            handlers: std::sync::Mutex::new(Vec::new()),
            history: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn event_history(&self) -> Vec<LivingProtocolEvent> {
        self.history.lock().unwrap().clone()
    }

    pub fn event_count(&self) -> usize {
        self.history.lock().unwrap().len()
    }
}

impl Default for InMemoryEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus for InMemoryEventBus {
    fn publish(&self, event: LivingProtocolEvent) {
        let handlers = self.handlers.lock().unwrap();
        for handler in handlers.iter() {
            handler(&event);
        }
        self.history.lock().unwrap().push(event);
    }

    fn subscribe(&self, handler: Box<dyn Fn(&LivingProtocolEvent) + Send + Sync>) {
        self.handlers.lock().unwrap().push(handler);
    }
}
