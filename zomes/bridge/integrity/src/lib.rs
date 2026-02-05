use hdi::prelude::*;
use mycelix_shared::{CyclePhase, Did, EpistemicClassification};

// ============================================================================
// Mycelix v6.0 Bridge Zome — Integrity Types
// ============================================================================
// Types for cross-DNA communication between living-protocol and mycelix-property.
// See INTEGRATION.md for the full integration architecture.

/// DNA names for cross-DNA calls.
pub const PROPERTY_DNA_NAME: &str = "mycelix-property";
pub const LIVING_PROTOCOL_DNA_NAME: &str = "mycelix-living-protocol";

// ============================================================================
// MATL Integration Types
// ============================================================================

/// Request to fetch MATL score from mycelix-property.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MatlScoreRequest {
    pub agent: AgentPubKey,
}

/// Response containing MATL score from mycelix-property.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MatlScoreResponse {
    pub agent: AgentPubKey,
    pub matl_score: f64,
    pub throughput_component: f64,
    pub resilience_component: f64,
    pub composting_component: f64,
    pub timestamp: Timestamp,
}

// ============================================================================
// Slash -> Wound Migration Types
// ============================================================================

/// Intercepted slash event from v5.2.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SlashInterceptEvent {
    pub offender: AgentPubKey,
    pub slash_percentage: f64,
    pub reason: String,
    pub original_action_hash: ActionHash,
}

/// Wound record created from intercepted slash.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WoundFromSlash {
    pub agent: AgentPubKey,
    pub severity: WoundSeverity,
    pub source_slash_action: ActionHash,
    pub escrow_address: Option<String>, // Ethereum escrow address if applicable
    pub healing_started: Timestamp,
}

/// Wound severity derived from slash percentage.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum WoundSeverity {
    Minor,      // 1-5% slash
    Moderate,   // 5-15% slash
    Severe,     // 15-30% slash
    Critical,   // 30%+ slash
}

impl WoundSeverity {
    pub fn from_slash_percentage(pct: f64) -> Self {
        if pct >= 0.30 {
            Self::Critical
        } else if pct >= 0.15 {
            Self::Severe
        } else if pct >= 0.05 {
            Self::Moderate
        } else {
            Self::Minor
        }
    }

    pub fn estimated_healing_days(&self) -> u32 {
        match self {
            Self::Minor => 3,
            Self::Moderate => 7,
            Self::Severe => 14,
            Self::Critical => 28,
        }
    }
}

// ============================================================================
// K-Vector Integration Types
// ============================================================================

/// Request to fetch K-Vector snapshot from mycelix-property.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KVectorSnapshotRequest {
    pub agent: AgentPubKey,
    pub as_of: Option<Timestamp>,
}

/// K-Vector snapshot response (v5.2 5-dimensional K-Vector).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KVectorSnapshotResponse {
    pub agent: AgentPubKey,
    pub dimensions: KVectorDimensions,
    pub timestamp: Timestamp,
    pub action_hash: ActionHash,
}

/// The 5 K-Vector dimensions from v5.2.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KVectorDimensions {
    pub stability: f64,
    pub adaptability: f64,
    pub integrity: f64,
    pub connectivity: f64,
    pub emergence: f64,
}

// ============================================================================
// Beauty Scoring Integration Types
// ============================================================================

/// Request to attach beauty score to a governance proposal.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttachBeautyScoreRequest {
    pub proposal_action_hash: ActionHash,
    pub beauty_score: BeautyScorePayload,
}

/// Beauty score payload to attach to governance records.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BeautyScorePayload {
    pub simplicity: f64,
    pub clarity: f64,
    pub resonance: f64,
    pub integration: f64,
    pub emergence: f64,
    pub composite_score: f64,
    pub cycle_phase: CyclePhase,
    pub scorer: AgentPubKey,
    pub timestamp: Timestamp,
}

// ============================================================================
// Agent Registry Integration Types
// ============================================================================

/// Request to resolve a DID to agent info.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DidResolveRequest {
    pub did: Did,
}

/// DID resolution response from agent registry.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DidResolveResponse {
    pub did: Did,
    pub agent_pub_key: AgentPubKey,
    pub display_name: Option<String>,
    pub verified: bool,
}

// ============================================================================
// Bridge Link Types
// ============================================================================

/// Link type for bridge connections.
#[hdk_link_types]
pub enum BridgeLinkTypes {
    /// Links to wound records created from slashes.
    SlashToWound,
    /// Links to beauty scores attached to proposals.
    ProposalToBeautyScore,
    /// Links K-Vector snapshots to temporal analysis.
    KVectorToTemporal,
}

// ============================================================================
// Bridge Entry Types
// ============================================================================

/// Record of a cross-DNA bridge call for auditability.
#[hdk_entry_helper]
#[derive(Clone)]
pub struct BridgeCallRecord {
    pub source_dna: String,
    pub target_dna: String,
    pub function_name: String,
    pub caller: AgentPubKey,
    pub timestamp: Timestamp,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Cached MATL score for local access.
#[hdk_entry_helper]
#[derive(Clone)]
pub struct CachedMatlScore {
    pub agent: AgentPubKey,
    pub matl_score: f64,
    pub throughput_component: f64,
    pub resilience_component: f64,
    pub composting_component: f64,
    pub fetched_at: Timestamp,
    pub expires_at: Timestamp,
}

/// Entry types for the bridge integrity zome.
#[hdk_entry_types]
#[unit_enum(BridgeUnitEntryTypes)]
pub enum BridgeEntryTypes {
    #[entry_type(name = "bridge_call_record", visibility = "public")]
    BridgeCallRecord(BridgeCallRecord),
    #[entry_type(name = "cached_matl_score", visibility = "private")]
    CachedMatlScore(CachedMatlScore),
}

// ============================================================================
// Validation
// ============================================================================

#[hdk_extern]
pub fn validate(_op: Op) -> ExternResult<ValidateCallbackResult> {
    // Bridge entries are primarily for audit and caching.
    // Core validation is handled by the individual domain zomes.
    Ok(ValidateCallbackResult::Valid)
}
