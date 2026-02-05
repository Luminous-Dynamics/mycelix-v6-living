use hdi::prelude::*;
use mycelix_shared::EpistemicClassification;

// ============================================================================
// Mycelix v6.0 — Living Consciousness Integrity Zome
// ============================================================================
// Primitives [5]-[8]: Temporal K-Vector, Field Interference, Dream Proposal,
// Network Phi Measurement

// ---------------------------------------------------------------------------
// Entry Types
// ---------------------------------------------------------------------------

/// [5] TemporalKVectorSnapshot — captures a snapshot of an agent's
/// 8-dimensional consciousness vector at a point in time.
/// Each dimension value must be in [0.0, 1.0].
///
/// Dimensions:
///  0: Presence    1: Coherence    2: Receptivity    3: Integration
///  4: Generativity 5: Surrender   6: Discernment    7: Emergence
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TemporalKVectorSnapshot {
    pub agent: AgentPubKey,
    pub dimensions: Vec<f64>, // exactly 8 values, each in [0.0, 1.0]
    pub context: String,
    pub epistemic: EpistemicClassification,
    pub captured_at: Timestamp,
}

/// [6] FieldInterferenceRecord — records observed interference between
/// two or more consciousness fields (constructive or destructive).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FieldInterferenceRecord {
    pub participants: Vec<AgentPubKey>,
    pub interference_type: InterferenceType,
    pub amplitude: f64,
    pub description: String,
    pub epistemic: EpistemicClassification,
    pub observed_at: Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InterferenceType {
    Constructive,
    Destructive,
    Mixed,
}

/// [7] DreamProposal — a proposal that emerged from dream-state or
/// liminal consciousness. Financial operations are explicitly forbidden.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DreamProposal {
    pub proposer: AgentPubKey,
    pub title: String,
    pub description: String,
    pub financial_operations: bool, // must be false
    pub confirmations: Vec<AgentPubKey>,
    pub rejection_count: u32,
    pub epistemic: EpistemicClassification,
    pub proposed_at: Timestamp,
}

/// [8] NetworkPhiMeasurement — measures integrated information (phi) for
/// a subset of the network, inspired by IIT.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct NetworkPhiMeasurement {
    pub measured_agents: Vec<AgentPubKey>,
    pub phi: f64, // >= 0.0
    pub measurement_method: String,
    pub contributing_factors: Vec<String>,
    pub epistemic: EpistemicClassification,
    pub measured_at: Timestamp,
}

// ---------------------------------------------------------------------------
// Entry / Link Enums
// ---------------------------------------------------------------------------

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    #[entry_type(name = "TemporalKVectorSnapshot")]
    TemporalKVectorSnapshot(TemporalKVectorSnapshot),
    #[entry_type(name = "FieldInterferenceRecord")]
    FieldInterferenceRecord(FieldInterferenceRecord),
    #[entry_type(name = "DreamProposal")]
    DreamProposal(DreamProposal),
    #[entry_type(name = "NetworkPhiMeasurement")]
    NetworkPhiMeasurement(NetworkPhiMeasurement),
}

#[hdk_link_types]
pub enum LinkTypes {
    AgentToKVectorHistory,
    DreamToConfirmation,
    PhiToAgents,
}

// ---------------------------------------------------------------------------
// Genesis Self-Check
// ---------------------------------------------------------------------------

#[hdk_extern]
pub fn genesis_self_check(_data: GenesisSelfCheckData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        // -- Store Entry --
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } | OpEntry::UpdateEntry { app_entry, .. } => {
                match app_entry {
                    EntryTypes::TemporalKVectorSnapshot(snapshot) => {
                        validate_k_vector_snapshot(&snapshot)
                    }
                    EntryTypes::FieldInterferenceRecord(record) => {
                        validate_field_interference(&record)
                    }
                    EntryTypes::DreamProposal(proposal) => validate_dream_proposal(&proposal),
                    EntryTypes::NetworkPhiMeasurement(measurement) => {
                        validate_network_phi(&measurement)
                    }
                }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // -- Store Record --
        FlatOp::StoreRecord(store_record) => match store_record {
            OpRecord::CreateEntry { app_entry, .. }
            | OpRecord::UpdateEntry { app_entry, .. } => match app_entry {
                EntryTypes::TemporalKVectorSnapshot(snapshot) => {
                    validate_k_vector_snapshot(&snapshot)
                }
                EntryTypes::FieldInterferenceRecord(record) => {
                    validate_field_interference(&record)
                }
                EntryTypes::DreamProposal(proposal) => validate_dream_proposal(&proposal),
                EntryTypes::NetworkPhiMeasurement(measurement) => {
                    validate_network_phi(&measurement)
                }
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // -- Register Links --
        FlatOp::RegisterCreateLink {
            link_type,
            ..
        } => match link_type {
            LinkTypes::AgentToKVectorHistory => Ok(ValidateCallbackResult::Valid),
            LinkTypes::DreamToConfirmation => Ok(ValidateCallbackResult::Valid),
            LinkTypes::PhiToAgents => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDeleteLink { .. } => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterUpdate(_) => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterDelete(_) => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterAgentActivity(_) => Ok(ValidateCallbackResult::Valid),
    }
}

// ---------------------------------------------------------------------------
// Validation Helpers
// ---------------------------------------------------------------------------

fn validate_k_vector_snapshot(
    snapshot: &TemporalKVectorSnapshot,
) -> ExternResult<ValidateCallbackResult> {
    if snapshot.dimensions.len() != 8 {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "TemporalKVectorSnapshot: dimensions must have exactly 8 values, got {}",
            snapshot.dimensions.len()
        )));
    }
    for (i, val) in snapshot.dimensions.iter().enumerate() {
        if !(0.0..=1.0).contains(val) {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "TemporalKVectorSnapshot: dimension {} value {} is out of [0.0, 1.0]",
                i, val
            )));
        }
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_field_interference(
    record: &FieldInterferenceRecord,
) -> ExternResult<ValidateCallbackResult> {
    if record.participants.len() < 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "FieldInterferenceRecord: requires at least 2 participants".to_string(),
        ));
    }
    if record.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "FieldInterferenceRecord: description must not be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_dream_proposal(proposal: &DreamProposal) -> ExternResult<ValidateCallbackResult> {
    if proposal.financial_operations {
        return Ok(ValidateCallbackResult::Invalid(
            "DreamProposal: financial_operations must be false — dreams cannot involve financial operations".to_string(),
        ));
    }
    if proposal.title.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "DreamProposal: title must not be empty".to_string(),
        ));
    }
    if proposal.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "DreamProposal: description must not be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_network_phi(
    measurement: &NetworkPhiMeasurement,
) -> ExternResult<ValidateCallbackResult> {
    if measurement.phi < 0.0 {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "NetworkPhiMeasurement: phi must be >= 0.0, got {}",
            measurement.phi
        )));
    }
    if measurement.measured_agents.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "NetworkPhiMeasurement: must include at least one measured agent".to_string(),
        ));
    }
    if measurement.measurement_method.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "NetworkPhiMeasurement: measurement_method must not be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
