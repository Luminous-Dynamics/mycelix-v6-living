use hdi::prelude::*;
use mycelix_shared::{CyclePhase, Did, EpistemicClassification};

// ============================================================================
// Mycelix v6.0 — Living Metabolism Integrity Zome
// ============================================================================
// Primitives [1]-[4]: Composting, Wound, Metabolic Trust, Kenosis

// ---------------------------------------------------------------------------
// Entry Types
// ---------------------------------------------------------------------------

/// [1] CompostingRecord — tracks the decomposition of outdated patterns,
/// beliefs, or structures so their nutrients can feed new growth.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CompostingRecord {
    pub agent: AgentPubKey,
    pub source_description: String,
    pub decomposition_progress: f64, // [0.0, 1.0]
    pub nutrients_released: Vec<String>,
    pub cycle_phase: CyclePhase,
    pub epistemic: EpistemicClassification,
    pub created_at: Timestamp,
}

/// Wound healing phases — must advance forward only.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum WoundPhase {
    Hemostasis,
    Inflammation,
    Proliferation,
    Remodeling,
    Healed,
}

impl WoundPhase {
    pub fn ordinal(&self) -> u8 {
        match self {
            WoundPhase::Hemostasis => 0,
            WoundPhase::Inflammation => 1,
            WoundPhase::Proliferation => 2,
            WoundPhase::Remodeling => 3,
            WoundPhase::Healed => 4,
        }
    }
}

/// [2] WoundRecord — tracks relational or systemic wounds through a
/// biological healing arc. Phase must only advance forward.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct WoundRecord {
    pub agent: AgentPubKey,
    pub description: String,
    pub phase: WoundPhase,
    pub witnesses: Vec<AgentPubKey>,
    pub restitution_offered: bool,
    pub epistemic: EpistemicClassification,
    pub created_at: Timestamp,
}

/// [3] MetabolicTrustRecord — a metabolic (earned, living) trust score
/// between two agents, bounded to [0.0, 1.0].
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MetabolicTrustRecord {
    pub from_agent: AgentPubKey,
    pub to_agent: AgentPubKey,
    pub score: f64, // [0.0, 1.0]
    pub context: String,
    pub evidence_hashes: Vec<ActionHash>,
    pub updated_at: Timestamp,
}

/// [4] KenosisCommitment — an irrevocable self-emptying commitment.
/// Release percentage is capped at 20% to prevent self-destructive acts.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct KenosisCommitment {
    pub agent: AgentPubKey,
    pub release_description: String,
    pub release_percentage: f64, // <= 0.20
    pub irrevocable: bool,
    pub beneficiaries: Vec<AgentPubKey>,
    pub epistemic: EpistemicClassification,
    pub created_at: Timestamp,
}

// ---------------------------------------------------------------------------
// Entry / Link Enums
// ---------------------------------------------------------------------------

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    #[entry_type(name = "CompostingRecord")]
    CompostingRecord(CompostingRecord),
    #[entry_type(name = "WoundRecord")]
    WoundRecord(WoundRecord),
    #[entry_type(name = "MetabolicTrustRecord")]
    MetabolicTrustRecord(MetabolicTrustRecord),
    #[entry_type(name = "KenosisCommitment")]
    KenosisCommitment(KenosisCommitment),
}

#[hdk_link_types]
pub enum LinkTypes {
    AgentToWounds,
    AgentToComposting,
    AgentToKenosis,
    WoundToRestitution,
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
                    EntryTypes::CompostingRecord(record) => validate_composting(&record),
                    EntryTypes::WoundRecord(record) => validate_wound_record(&record),
                    EntryTypes::MetabolicTrustRecord(record) => {
                        validate_metabolic_trust(&record)
                    }
                    EntryTypes::KenosisCommitment(record) => {
                        validate_kenosis_commitment(&record)
                    }
                }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // -- Store Record --
        FlatOp::StoreRecord(store_record) => match store_record {
            OpRecord::CreateEntry { app_entry, .. }
            | OpRecord::UpdateEntry { app_entry, .. } => match app_entry {
                EntryTypes::CompostingRecord(record) => validate_composting(&record),
                EntryTypes::WoundRecord(record) => validate_wound_record(&record),
                EntryTypes::MetabolicTrustRecord(record) => {
                    validate_metabolic_trust(&record)
                }
                EntryTypes::KenosisCommitment(record) => {
                    validate_kenosis_commitment(&record)
                }
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // -- Register Links --
        FlatOp::RegisterCreateLink {
            link_type,
            base_address: _,
            target_address: _,
            tag: _,
        } => match link_type {
            LinkTypes::AgentToWounds => Ok(ValidateCallbackResult::Valid),
            LinkTypes::AgentToComposting => Ok(ValidateCallbackResult::Valid),
            LinkTypes::AgentToKenosis => Ok(ValidateCallbackResult::Valid),
            LinkTypes::WoundToRestitution => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDeleteLink {
            link_type,
            original_action: _,
            base_address: _,
            target_address: _,
            tag: _,
        } => match link_type {
            // Kenosis links are irrevocable — cannot be deleted
            LinkTypes::AgentToKenosis => Ok(ValidateCallbackResult::Invalid(
                "Kenosis links are irrevocable and cannot be deleted".to_string(),
            )),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // -- Everything else passes --
        FlatOp::RegisterUpdate(update) => match update {
            OpUpdate::Entry { app_entry, .. } => match app_entry {
                EntryTypes::KenosisCommitment(_) => Ok(ValidateCallbackResult::Invalid(
                    "KenosisCommitment entries are irrevocable and cannot be updated".to_string(),
                )),
                _ => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDelete(_) => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterAgentActivity(_) => Ok(ValidateCallbackResult::Valid),
    }
}

// ---------------------------------------------------------------------------
// Validation Helpers
// ---------------------------------------------------------------------------

fn validate_composting(record: &CompostingRecord) -> ExternResult<ValidateCallbackResult> {
    if !(0.0..=1.0).contains(&record.decomposition_progress) {
        return Ok(ValidateCallbackResult::Invalid(
            "CompostingRecord: decomposition_progress must be in [0.0, 1.0]".to_string(),
        ));
    }
    if record.source_description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "CompostingRecord: source_description must not be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_wound_record(record: &WoundRecord) -> ExternResult<ValidateCallbackResult> {
    // Phase ordering is enforced at update time by the coordinator; here we
    // just validate the record in isolation.
    if record.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "WoundRecord: description must not be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_metabolic_trust(record: &MetabolicTrustRecord) -> ExternResult<ValidateCallbackResult> {
    if !(0.0..=1.0).contains(&record.score) {
        return Ok(ValidateCallbackResult::Invalid(
            "MetabolicTrustRecord: score must be in [0.0, 1.0]".to_string(),
        ));
    }
    if record.from_agent == record.to_agent {
        return Ok(ValidateCallbackResult::Invalid(
            "MetabolicTrustRecord: cannot assign metabolic trust to yourself".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_kenosis_commitment(record: &KenosisCommitment) -> ExternResult<ValidateCallbackResult> {
    if record.release_percentage > 0.20 {
        return Ok(ValidateCallbackResult::Invalid(
            "KenosisCommitment: release_percentage must be <= 0.20 (20% cap)".to_string(),
        ));
    }
    if record.release_percentage < 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "KenosisCommitment: release_percentage must be >= 0.0".to_string(),
        ));
    }
    if !record.irrevocable {
        return Ok(ValidateCallbackResult::Invalid(
            "KenosisCommitment: irrevocable flag must be true".to_string(),
        ));
    }
    if record.beneficiaries.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "KenosisCommitment: must have at least one beneficiary".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
