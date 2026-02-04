//! Error types for the Living Protocol Layer.

use thiserror::Error;

use crate::types::{CyclePhase, Did};

#[derive(Error, Debug)]
pub enum LivingProtocolError {
    // Metabolism errors
    #[error("Entity {0} is not eligible for composting: {1}")]
    CompostingIneligible(String, String),

    #[error("Wound healing phase violation: cannot transition from {from:?} to {to:?}")]
    WoundPhaseViolation {
        from: crate::types::WoundPhase,
        to: crate::types::WoundPhase,
    },

    #[error("Metabolic trust score out of bounds: {0} (must be [0.0, 1.0])")]
    MetabolicTrustOutOfBounds(f64),

    #[error("Kenosis cap exceeded: {attempted:.2}% exceeds {max:.2}% per cycle")]
    KenosisCapExceeded { attempted: f64, max: f64 },

    #[error("Kenosis already committed and is irrevocable")]
    KenosisIrrevocable,

    // Consciousness errors
    #[error("K-Vector dimension mismatch: expected {expected}, got {got}")]
    KVectorDimensionMismatch { expected: usize, got: usize },

    #[error("Field interference requires at least 2 K-Vector fields")]
    InsufficientFieldsForInterference,

    #[error("Operation not permitted during dream phase: {0}")]
    DreamPhaseRestriction(String),

    #[error("Phi computation failed: {0}")]
    PhiComputationFailed(String),

    // Epistemic errors
    #[error("Claim {0} is held in uncertainty and cannot be voted on")]
    VotingBlockedByUncertainty(String),

    #[error("Silence requires valid PresenceProof: {0}")]
    InvalidPresenceProof(String),

    #[error("Shadow integration cannot surface Gate 1 protected content")]
    ShadowGate1Violation,

    #[error("Beauty score component out of range: {component} = {value}")]
    BeautyScoreOutOfRange { component: String, value: f64 },

    // Relational errors
    #[error("Entanglement requires shared co-creation history")]
    EntanglementRequiresHistory,

    #[error("Agent {0} is not in liminal state")]
    NotInLiminalState(Did),

    #[error("Inter-species bridge protocol mismatch: {0}")]
    InterSpeciesProtocolMismatch(String),

    // Structural errors
    #[error("Resonance address pattern invalid: {0}")]
    InvalidResonancePattern(String),

    #[error("Fractal governance scale mismatch: {0}")]
    FractalScaleMismatch(String),

    #[error("Time-crystal period violation: {0}")]
    TimeCrystalPeriodViolation(String),

    // Cycle errors
    #[error("Invalid cycle phase transition: {from:?} -> {to:?}")]
    InvalidPhaseTransition { from: CyclePhase, to: CyclePhase },

    #[error("Operation not permitted in current phase {phase:?}: {reason}")]
    PhaseRestriction { phase: CyclePhase, reason: String },

    #[error("Cycle not initialized")]
    CycleNotInitialized,

    // General
    #[error("Agent not found: {0}")]
    AgentNotFound(Did),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Cryptographic verification failed: {0}")]
    CryptoVerificationFailed(String),

    #[error("Feature not enabled: {0} (requires feature flag '{1}')")]
    FeatureNotEnabled(String, String),
}

pub type LivingResult<T> = Result<T, LivingProtocolError>;
