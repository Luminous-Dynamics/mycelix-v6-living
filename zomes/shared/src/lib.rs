use hdi::prelude::*;

// ============================================================================
// Mycelix v6.0 Living Protocol Layer — Shared Types
// ============================================================================
// Types used across all living protocol zomes for consistent interop.

/// Decentralized Identifier wrapper.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Did(pub String);

/// Epistemic classification triple used throughout the protocol.
/// - e_tier: evidential tier (0 = direct experience, higher = more mediated)
/// - n_tier: novelty tier (0 = well-known, higher = more novel)
/// - m_tier: maturity tier (0 = embryonic, higher = more mature)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EpistemicClassification {
    pub e_tier: u8,
    pub n_tier: u8,
    pub m_tier: u8,
}

/// The nine phases of the Mycelix living cycle.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CyclePhase {
    Shadow,
    Composting,
    Liminal,
    NegativeCapability,
    Eros,
    CoCreation,
    Beauty,
    EmergentPersonhood,
    Kenosis,
}

/// Proof that an agent was present/attentive during a period.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PresenceProof {
    pub agent: AgentPubKey,
    pub timestamp: Timestamp,
    pub proof_type: PresenceProofType,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PresenceProofType {
    HeartbeatSignal,
    WitnessAttestation { witness: AgentPubKey },
    ActivityTrace { action_hash: ActionHash },
}

/// Shared helper: clamp a float to [0.0, 1.0].
pub fn is_unit_interval(v: f64) -> bool {
    (0.0..=1.0).contains(&v)
}

/// Shared helper: check a float is non-negative.
pub fn is_non_negative(v: f64) -> bool {
    v >= 0.0
}
