//! Core types shared across all Living Protocol primitives.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Duration, Utc};

// =============================================================================
// Identity Types
// =============================================================================

/// Decentralized Identifier (W3C DID)
pub type Did = String;

/// Unique claim identifier
pub type ClaimId = String;

/// Unique entity identifier within the living protocol
pub type EntityId = String;

/// Cryptographic signature bytes
pub type SignatureBytes = Vec<u8>;

/// Hash digest
pub type HashDigest = [u8; 32];

// =============================================================================
// Epistemic Classification (E/N/M)
// =============================================================================

/// Epistemic tier - how certain are we?
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EpistemicTier {
    /// E0: Null - no epistemic basis
    Null = 0,
    /// E1: Testimonial - someone said so
    Testimonial = 1,
    /// E2: Privately verifiable - tested multiple times
    PrivatelyVerifiable = 2,
    /// E3: Cryptographically proven
    CryptographicallyProven = 3,
    /// E4: Formally verified
    FormallyVerified = 4,
}

/// Normative tier - how widely applicable?
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum NormativeTier {
    /// N0: Personal
    Personal = 0,
    /// N1: Communal
    Communal = 1,
    /// N2: Network consensus
    NetworkConsensus = 2,
    /// N3: Axiomatic
    Axiomatic = 3,
}

/// Materiality tier - how persistent?
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MaterialityTier {
    /// M0: Ephemeral
    Ephemeral = 0,
    /// M1: Temporal
    Temporal = 1,
    /// M2: Persistent
    Persistent = 2,
    /// M3: Foundational
    Foundational = 3,
}

/// Complete epistemic classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpistemicClassification {
    pub e: EpistemicTier,
    pub n: NormativeTier,
    pub m: MaterialityTier,
}

// =============================================================================
// Claim Status (extended for v6.0)
// =============================================================================

/// Status of an epistemic claim in the DKG.
/// v6.0 adds `HeldInUncertainty` for Negative Capability [10].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimStatus {
    /// Active and accepted
    Active,
    /// Disputed but unresolved
    Disputed,
    /// Resolved in favor
    Resolved,
    /// Refuted by evidence
    Refuted,
    /// Superseded by newer claim
    Superseded,
    /// v6.0: Held in uncertainty - voting blocked, question remains open
    HeldInUncertainty {
        reason: String,
        held_since: DateTime<Utc>,
        /// Earliest time this can be forced to resolution
        earliest_resolution: DateTime<Utc>,
    },
    /// v6.0: Being composted - extracting learnings before removal
    Composting {
        started: DateTime<Utc>,
        nutrients_extracted: Vec<String>,
    },
    /// v6.0: In shadow - suppressed content surfaced for review
    InShadow {
        surfaced: DateTime<Utc>,
        original_suppression: DateTime<Utc>,
    },
}

// =============================================================================
// Metabolism Cycle Types
// =============================================================================

/// Phases of the 28-day Metabolism Cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CyclePhase {
    /// Shadow (2 days): Suppression detection, dissent surfaces
    Shadow,
    /// Composting (5 days): Failed entities decomposed, nutrients extracted
    Composting,
    /// Liminal (3 days): Transitioning entities in threshold state
    Liminal,
    /// Negative Capability (3 days): Open questions held, voting blocked
    NegativeCapability,
    /// Eros (4 days): Attractor fields computed, complementary agents connected
    Eros,
    /// Co-Creation (7 days): Standard consensus, entangled pairs form
    CoCreation,
    /// Beauty (2 days): Proposals scored on aesthetic criteria
    Beauty,
    /// Emergent Personhood (1 day): Network self-measurement
    EmergentPersonhood,
    /// Kenosis (1 day): Voluntary reputation release
    Kenosis,
}

impl CyclePhase {
    /// Duration of this phase in days.
    pub fn duration_days(&self) -> u32 {
        match self {
            Self::Shadow => 2,
            Self::Composting => 5,
            Self::Liminal => 3,
            Self::NegativeCapability => 3,
            Self::Eros => 4,
            Self::CoCreation => 7,
            Self::Beauty => 2,
            Self::EmergentPersonhood => 1,
            Self::Kenosis => 1,
        }
    }

    /// Duration as chrono::Duration.
    pub fn duration(&self) -> Duration {
        Duration::days(self.duration_days() as i64)
    }

    /// Next phase in the cycle (wraps around).
    pub fn next(&self) -> Self {
        match self {
            Self::Shadow => Self::Composting,
            Self::Composting => Self::Liminal,
            Self::Liminal => Self::NegativeCapability,
            Self::NegativeCapability => Self::Eros,
            Self::Eros => Self::CoCreation,
            Self::CoCreation => Self::Beauty,
            Self::Beauty => Self::EmergentPersonhood,
            Self::EmergentPersonhood => Self::Kenosis,
            Self::Kenosis => Self::Shadow,
        }
    }

    /// Previous phase in the cycle (wraps around).
    pub fn prev(&self) -> Self {
        match self {
            Self::Shadow => Self::Kenosis,
            Self::Composting => Self::Shadow,
            Self::Liminal => Self::Composting,
            Self::NegativeCapability => Self::Liminal,
            Self::Eros => Self::NegativeCapability,
            Self::CoCreation => Self::Eros,
            Self::Beauty => Self::CoCreation,
            Self::EmergentPersonhood => Self::Beauty,
            Self::Kenosis => Self::EmergentPersonhood,
        }
    }

    /// All phases in cycle order.
    pub fn all_phases() -> &'static [CyclePhase] {
        &[
            Self::Shadow,
            Self::Composting,
            Self::Liminal,
            Self::NegativeCapability,
            Self::Eros,
            Self::CoCreation,
            Self::Beauty,
            Self::EmergentPersonhood,
            Self::Kenosis,
        ]
    }

    /// Total cycle length in days (should always be 28).
    pub fn total_cycle_days() -> u32 {
        Self::all_phases().iter().map(|p| p.duration_days()).sum()
    }
}

/// Current state of a metabolism cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleState {
    pub cycle_number: u64,
    pub current_phase: CyclePhase,
    pub phase_started: DateTime<Utc>,
    pub cycle_started: DateTime<Utc>,
    pub phase_day: u32,
}

impl CycleState {
    /// Whether the current phase has expired and should transition.
    pub fn phase_expired(&self, now: DateTime<Utc>) -> bool {
        let elapsed = now - self.phase_started;
        elapsed >= self.current_phase.duration()
    }

    /// Time remaining in current phase.
    pub fn time_remaining(&self, now: DateTime<Utc>) -> Duration {
        let end = self.phase_started + self.current_phase.duration();
        if now >= end {
            Duration::zero()
        } else {
            end - now
        }
    }
}

// =============================================================================
// Wound Healing Types
// =============================================================================

/// Phases of the wound healing process (replaces punitive slashing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WoundPhase {
    /// Immediate quarantine - automatic, un-gameable
    Hemostasis,
    /// Community assessment of damage
    Inflammation,
    /// Restitution and repair
    Proliferation,
    /// Integration and strengthening (scar tissue)
    Remodeling,
    /// Fully healed
    Healed,
}

impl WoundPhase {
    /// Valid next phases (forward-only).
    pub fn valid_transitions(&self) -> &[WoundPhase] {
        match self {
            Self::Hemostasis => &[Self::Inflammation],
            Self::Inflammation => &[Self::Proliferation],
            Self::Proliferation => &[Self::Remodeling],
            Self::Remodeling => &[Self::Healed],
            Self::Healed => &[],
        }
    }

    /// Whether transition to the given phase is valid.
    pub fn can_transition_to(&self, target: &WoundPhase) -> bool {
        self.valid_transitions().contains(target)
    }
}

/// Severity of a wound, mapped from existing slash percentages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WoundSeverity {
    /// Minor (was 1-5% slash) - heals in 1 cycle
    Minor,
    /// Moderate (was 5-15% slash) - heals in 2-3 cycles
    Moderate,
    /// Severe (was 15-30% slash) - heals in 4-6 cycles
    Severe,
    /// Critical (was 30%+ slash) - heals in 7+ cycles, may leave permanent scar
    Critical,
}

/// A wound record tracking the healing process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WoundRecord {
    pub id: String,
    pub agent_did: Did,
    pub severity: WoundSeverity,
    pub cause: String,
    pub phase: WoundPhase,
    pub created: DateTime<Utc>,
    pub phase_history: Vec<(WoundPhase, DateTime<Utc>)>,
    pub restitution_required: Option<RestitutionRequirement>,
    pub scar_tissue: Option<ScarTissue>,
}

/// What restitution is required for wound healing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestitutionRequirement {
    pub description: String,
    pub amount_flow: Option<f64>,
    pub actions_required: Vec<String>,
    pub deadline: DateTime<Utc>,
    pub fulfilled: bool,
}

/// Scar tissue: strengthening that results from healing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScarTissue {
    pub area: String,
    pub strength_multiplier: f64,
    pub formed: DateTime<Utc>,
}

// =============================================================================
// Metabolic Trust Types
// =============================================================================

/// Metabolic trust score extending MATL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetabolicTrustScore {
    /// Standard MATL composite: 0.4*PoGQ + 0.3*TCDM + 0.3*entropy
    pub matl_composite: f64,
    /// Metabolic throughput: rate of useful work
    pub throughput: f64,
    /// Recovery resilience: how well agent recovers from wounds
    pub resilience: f64,
    /// Composting contribution: nutrients given back to network
    pub composting_contribution: f64,
    /// Final metabolic trust score [0.0, 1.0]
    pub metabolic_trust: f64,
    pub computed_at: DateTime<Utc>,
}

impl MetabolicTrustScore {
    /// Compute metabolic trust from components.
    pub fn compute(
        matl_composite: f64,
        throughput: f64,
        resilience: f64,
        composting_contribution: f64,
    ) -> Self {
        // Extended formula:
        // T_metabolic = 0.35*MATL + 0.25*throughput + 0.20*resilience + 0.20*composting
        let metabolic_trust = (0.35 * matl_composite
            + 0.25 * throughput
            + 0.20 * resilience
            + 0.20 * composting_contribution)
            .clamp(0.0, 1.0);

        Self {
            matl_composite,
            throughput,
            resilience,
            composting_contribution,
            metabolic_trust,
            computed_at: Utc::now(),
        }
    }
}

// =============================================================================
// Kenosis Types
// =============================================================================

/// A kenosis (self-emptying) commitment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KenosisCommitment {
    pub id: String,
    pub agent_did: Did,
    /// Percentage of reputation to release (max 20% per cycle)
    pub release_percentage: f64,
    /// The reputation amount being released
    pub reputation_released: f64,
    pub committed_at: DateTime<Utc>,
    pub cycle_number: u64,
    /// Irrevocable once committed
    pub irrevocable: bool,
}

// =============================================================================
// Composting Types
// =============================================================================

/// An entity being composted (decomposed for nutrients).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompostingRecord {
    pub id: String,
    pub entity_type: CompostableEntity,
    pub entity_id: EntityId,
    pub started: DateTime<Utc>,
    pub nutrients: Vec<Nutrient>,
    pub decomposition_progress: f64,
    pub completed: Option<DateTime<Utc>>,
}

/// Types of entities that can be composted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompostableEntity {
    /// Failed proposal
    FailedProposal,
    /// Abandoned project
    AbandonedProject,
    /// Expired claim
    ExpiredClaim,
    /// Deprecated protocol component
    DeprecatedComponent,
    /// Dissolved DAO
    DissolvedDao,
}

/// A nutrient extracted from composting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nutrient {
    pub id: String,
    pub source_entity: EntityId,
    pub learning: String,
    pub classification: EpistemicClassification,
    pub extracted_at: DateTime<Utc>,
    /// Published to DKG?
    pub published: bool,
}

// =============================================================================
// Dreaming Types
// =============================================================================

/// Collective dreaming states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DreamState {
    /// Normal waking operation
    Waking,
    /// REM: Pattern recombination, creative exploration
    Rem,
    /// Deep: Memory consolidation, structural optimization
    Deep,
    /// Lucid: Conscious dreaming, guided exploration
    Lucid,
}

/// A dream proposal generated during dream phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamProposal {
    pub id: String,
    pub dream_state: DreamState,
    pub content: String,
    pub generated_at: DateTime<Utc>,
    /// Non-binding until confirmed during Waking
    pub confirmed: bool,
    /// Requires 0.67 threshold to confirm
    pub confirmation_threshold: f64,
    /// No financial operations allowed
    pub financial_operations: bool,
}

// =============================================================================
// Liminality Types
// =============================================================================

/// Liminal state for entities in transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiminalPhase {
    /// Pre-liminal: about to enter transition
    PreLiminal,
    /// Liminal: in the threshold, betwixt and between
    Liminal,
    /// Post-liminal: emerging from transition
    PostLiminal,
    /// Integrated: transition complete
    Integrated,
}

/// A liminal record tracking identity transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiminalRecord {
    pub id: String,
    pub entity_did: Did,
    pub entity_type: LiminalEntityType,
    pub phase: LiminalPhase,
    pub entered: DateTime<Utc>,
    pub previous_identity: Option<String>,
    pub emerging_identity: Option<String>,
    /// Cannot be prematurely recategorized
    pub recategorization_blocked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiminalEntityType {
    Agent,
    Dao,
    Protocol,
    Community,
}

// =============================================================================
// Beauty as Validity Types
// =============================================================================

/// Beauty score for proposal evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeautyScore {
    /// Structural balance and proportion
    pub symmetry: f64,
    /// Minimal complexity for the goal
    pub economy: f64,
    /// Harmonic alignment with existing patterns
    pub resonance: f64,
    /// Novel or unexpected elements
    pub surprise: f64,
    /// Coverage of requirements
    pub completeness: f64,
    /// Weighted composite [0.0, 1.0]
    pub composite: f64,
}

impl BeautyScore {
    /// Compute beauty score from components.
    pub fn compute(symmetry: f64, economy: f64, resonance: f64, surprise: f64, completeness: f64) -> Self {
        let composite = (0.20 * symmetry
            + 0.20 * economy
            + 0.25 * resonance
            + 0.15 * surprise
            + 0.20 * completeness)
            .clamp(0.0, 1.0);

        Self {
            symmetry,
            economy,
            resonance,
            surprise,
            completeness,
            composite,
        }
    }
}

// =============================================================================
// Silence as Signal Types
// =============================================================================

/// Proof of presence (can't fake being present to fake silence).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceProof {
    pub agent_did: Did,
    pub timestamp: DateTime<Utc>,
    pub heartbeat_hash: HashDigest,
    pub signature: SignatureBytes,
}

/// A meaningful silence record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilenceRecord {
    pub agent_did: Did,
    pub topic: String,
    pub silence_started: DateTime<Utc>,
    pub presence_proofs: Vec<PresenceProof>,
    pub classification: SilenceClassification,
}

/// Classification of silence type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SilenceClassification {
    /// Deliberate withholding - agent chooses not to speak
    DeliberateWithholding,
    /// Contemplative - agent is processing
    Contemplative,
    /// Dissent through absence - disagrees but doesn't want conflict
    DissentThroughAbsence,
    /// Unknown - insufficient data to classify
    Unknown,
}

// =============================================================================
// Entangled Pairs Types
// =============================================================================

/// An entangled pair of agents with correlated K-Vectors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntangledPair {
    pub id: String,
    pub agent_a: Did,
    pub agent_b: Did,
    /// Correlation strength [0.0, 1.0], decays without co-creation
    pub entanglement_strength: f64,
    pub formed: DateTime<Utc>,
    pub last_co_creation: DateTime<Utc>,
    /// Decay rate per day without interaction
    pub decay_rate: f64,
}

impl EntangledPair {
    /// Current entanglement strength accounting for decay.
    pub fn current_strength(&self, now: DateTime<Utc>) -> f64 {
        let days_since = (now - self.last_co_creation).num_days() as f64;
        let decayed = self.entanglement_strength * (-self.decay_rate * days_since).exp();
        decayed.max(0.0)
    }
}

// =============================================================================
// Shadow Integration Types
// =============================================================================

/// A shadow record - suppressed content surfaced for review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowRecord {
    pub id: String,
    pub original_content_id: String,
    pub suppressed_at: DateTime<Utc>,
    pub surfaced_at: DateTime<Utc>,
    pub suppression_reason: String,
    /// Was this a low-rep dissent that was drowned out?
    pub low_rep_dissent: bool,
    /// Spectral K anomaly that triggered surfacing
    pub spectral_k_anomaly: Option<f64>,
}

// =============================================================================
// Fractal Governance Types
// =============================================================================

/// A fractal governance pattern - structurally identical at all scales.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FractalGovernancePattern {
    pub id: String,
    pub scale: GovernanceScale,
    pub parent_pattern_id: Option<String>,
    pub child_patterns: Vec<String>,
    pub quorum_ratio: f64,
    pub supermajority_ratio: f64,
    pub decision_mechanism: DecisionMechanism,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernanceScale {
    Individual,
    Team,
    Community,
    Sector,
    Regional,
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionMechanism {
    Consent,
    Consensus,
    Supermajority,
    ReputationWeighted,
}

// =============================================================================
// Resonance Addressing Types
// =============================================================================

/// A resonance address - pattern-based routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceAddress {
    pub pattern_hash: HashDigest,
    pub semantic_embedding: Vec<f64>,
    pub harmonic_signature: Vec<f64>,
    pub created: DateTime<Utc>,
}

// =============================================================================
// Time-Crystal Consensus Types
// =============================================================================

/// Time-crystal consensus period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeCrystalPeriod {
    pub period_id: u64,
    pub phase_angle: f64,
    pub symmetry_group: String,
    pub validators: Vec<Did>,
    pub started: DateTime<Utc>,
    pub period_duration: Duration,
}

// =============================================================================
// Morphogenetic Field Types
// =============================================================================

/// A morphogenetic field guiding structural emergence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphogeneticField {
    pub id: String,
    pub field_type: FieldType,
    pub strength: f64,
    pub gradient: Vec<f64>,
    pub source_pattern_id: String,
    pub created: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    Attracting,
    Repelling,
    Guiding,
    Containing,
}

// =============================================================================
// Inter-Species Types
// =============================================================================

/// Inter-species participation protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterSpeciesParticipant {
    pub id: String,
    pub species: SpeciesType,
    pub bridge_protocol: String,
    pub capabilities: Vec<String>,
    pub constraints: Vec<String>,
    pub registered: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeciesType {
    Human,
    AiAgent,
    Dao,
    Sensor,
    Ecological,
    Other(String),
}

// =============================================================================
// Mycelial Computation Types
// =============================================================================

/// A mycelial computation task distributed across the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MycelialTask {
    pub id: String,
    pub computation: String,
    pub input_hash: HashDigest,
    pub assigned_nodes: Vec<Did>,
    pub result_hash: Option<HashDigest>,
    pub started: DateTime<Utc>,
    pub completed: Option<DateTime<Utc>>,
}

// =============================================================================
// Phase Transition Record
// =============================================================================

/// Record of a phase transition in the metabolism cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseTransition {
    pub from: CyclePhase,
    pub to: CyclePhase,
    pub cycle_number: u64,
    pub transitioned_at: DateTime<Utc>,
    pub metrics: PhaseMetrics,
}

/// Metrics collected at phase transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseMetrics {
    pub active_agents: u64,
    pub spectral_k: f64,
    pub mean_metabolic_trust: f64,
    pub active_wounds: u64,
    pub composting_entities: u64,
    pub liminal_entities: u64,
    pub entangled_pairs: u64,
    pub held_uncertainties: u64,
}

// =============================================================================
// Three Gates Types
// =============================================================================

/// Gate 1: Mathematical invariants (absolute enforcement).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate1Check {
    pub invariant: String,
    pub passed: bool,
    pub details: Option<String>,
}

/// Gate 2: Constitutional warnings (reputation impact).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate2Warning {
    pub harmony_violated: String,
    pub severity: f64,
    pub reputation_impact: f64,
    pub reasoning: String,
    pub user_may_proceed: bool,
}

/// Gate 3: Social consequences (emergent enforcement).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate3Consequence {
    pub action: String,
    pub attestations: Vec<(Did, f64)>,
    pub reputation_delta: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_phase_total_is_28() {
        assert_eq!(CyclePhase::total_cycle_days(), 28);
    }

    #[test]
    fn test_cycle_phase_transitions() {
        let phase = CyclePhase::Shadow;
        assert_eq!(phase.next(), CyclePhase::Composting);
        assert_eq!(phase.prev(), CyclePhase::Kenosis);
    }

    #[test]
    fn test_cycle_wraps_around() {
        let mut phase = CyclePhase::Shadow;
        for _ in 0..9 {
            phase = phase.next();
        }
        assert_eq!(phase, CyclePhase::Shadow);
    }

    #[test]
    fn test_wound_phase_forward_only() {
        let hemostasis = WoundPhase::Hemostasis;
        assert!(hemostasis.can_transition_to(&WoundPhase::Inflammation));
        assert!(!hemostasis.can_transition_to(&WoundPhase::Proliferation));
        assert!(!hemostasis.can_transition_to(&WoundPhase::Healed));
    }

    #[test]
    fn test_metabolic_trust_bounded() {
        let score = MetabolicTrustScore::compute(0.8, 0.9, 0.7, 0.6);
        assert!(score.metabolic_trust >= 0.0 && score.metabolic_trust <= 1.0);
    }

    #[test]
    fn test_metabolic_trust_clamped_high() {
        let score = MetabolicTrustScore::compute(1.5, 1.5, 1.5, 1.5);
        assert!(score.metabolic_trust <= 1.0);
    }

    #[test]
    fn test_beauty_score_bounded() {
        let score = BeautyScore::compute(0.8, 0.9, 0.7, 0.6, 0.85);
        assert!(score.composite >= 0.0 && score.composite <= 1.0);
    }

    #[test]
    fn test_entangled_pair_decay() {
        let pair = EntangledPair {
            id: "test".into(),
            agent_a: "did:a".into(),
            agent_b: "did:b".into(),
            entanglement_strength: 1.0,
            formed: Utc::now(),
            last_co_creation: Utc::now() - Duration::days(10),
            decay_rate: 0.1,
        };
        let strength = pair.current_strength(Utc::now());
        assert!(strength < 1.0);
        assert!(strength > 0.0);
    }
}
