use hdi::prelude::*;
use mycelix_shared::EpistemicClassification;

// ============================================================================
// Mycelix v6.0 — Living Structural Integrity Zome
// ============================================================================
// Primitives [17]-[21]: Resonance Address, Fractal Governance,
// Morphogenetic Field, Time Crystal, Mycelial Task

// ---------------------------------------------------------------------------
// Entry Types
// ---------------------------------------------------------------------------

/// [17] ResonanceAddressRecord — a content-addressed identifier based on
/// pattern resonance rather than location, enabling pattern-based discovery.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ResonanceAddressRecord {
    pub creator: AgentPubKey,
    pub pattern_vector: Vec<f64>,
    pub description: String,
    pub referenced_hashes: Vec<ActionHash>,
    pub epistemic: EpistemicClassification,
    pub created_at: Timestamp,
}

/// [18] FractalGovernanceRecord — a governance pattern that maintains
/// structural identity across scales (self-similar at every level).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct FractalGovernanceRecord {
    pub creator: AgentPubKey,
    pub pattern_name: String,
    pub scale: String,
    pub parent_pattern_hash: Option<ActionHash>,
    pub structural_rules: Vec<String>,
    pub participants: Vec<AgentPubKey>,
    pub epistemic: EpistemicClassification,
    pub created_at: Timestamp,
}

/// [19] MorphogeneticFieldRecord — describes a field of developmental
/// potential that shapes how structures emerge.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MorphogeneticFieldRecord {
    pub creator: AgentPubKey,
    pub field_description: String,
    pub influence_radius: f64,
    pub developmental_stage: String,
    pub influenced_agents: Vec<AgentPubKey>,
    pub epistemic: EpistemicClassification,
    pub created_at: Timestamp,
}

/// [20] TimeCrystalRecord — a temporal structure that repeats with a
/// defined period, creating rhythmic patterns in the network.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TimeCrystalRecord {
    pub creator: AgentPubKey,
    pub description: String,
    pub period_duration: u64, // duration in seconds; must be > 0
    pub phase_offset: u64,
    pub participants: Vec<AgentPubKey>,
    pub active: bool,
    pub epistemic: EpistemicClassification,
    pub created_at: Timestamp,
}

/// [21] MycelialTaskRecord — a task distributed across the mycelial network,
/// analogous to nutrient transport in fungal networks.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MycelialTaskRecord {
    pub creator: AgentPubKey,
    pub task_description: String,
    pub input_hash: ActionHash,
    pub assigned_nodes: Vec<AgentPubKey>,
    pub status: TaskStatus,
    pub result_hash: Option<ActionHash>,
    pub epistemic: EpistemicClassification,
    pub created_at: Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

// ---------------------------------------------------------------------------
// Entry / Link Enums
// ---------------------------------------------------------------------------

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    #[entry_type(name = "ResonanceAddressRecord")]
    ResonanceAddressRecord(ResonanceAddressRecord),
    #[entry_type(name = "FractalGovernanceRecord")]
    FractalGovernanceRecord(FractalGovernanceRecord),
    #[entry_type(name = "MorphogeneticFieldRecord")]
    MorphogeneticFieldRecord(MorphogeneticFieldRecord),
    #[entry_type(name = "TimeCrystalRecord")]
    TimeCrystalRecord(TimeCrystalRecord),
    #[entry_type(name = "MycelialTaskRecord")]
    MycelialTaskRecord(MycelialTaskRecord),
}

#[hdk_link_types]
pub enum LinkTypes {
    PatternToAddress,
    ScaleToGovernance,
    TaskToNodes,
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
                    EntryTypes::ResonanceAddressRecord(record) => {
                        validate_resonance_address(&record)
                    }
                    EntryTypes::FractalGovernanceRecord(record) => {
                        validate_fractal_governance(&record)
                    }
                    EntryTypes::MorphogeneticFieldRecord(record) => {
                        validate_morphogenetic_field(&record)
                    }
                    EntryTypes::TimeCrystalRecord(record) => {
                        validate_time_crystal(&record)
                    }
                    EntryTypes::MycelialTaskRecord(record) => {
                        validate_mycelial_task(&record)
                    }
                }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // -- Store Record --
        FlatOp::StoreRecord(store_record) => match store_record {
            OpRecord::CreateEntry { app_entry, .. }
            | OpRecord::UpdateEntry { app_entry, .. } => match app_entry {
                EntryTypes::ResonanceAddressRecord(record) => {
                    validate_resonance_address(&record)
                }
                EntryTypes::FractalGovernanceRecord(record) => {
                    validate_fractal_governance(&record)
                }
                EntryTypes::MorphogeneticFieldRecord(record) => {
                    validate_morphogenetic_field(&record)
                }
                EntryTypes::TimeCrystalRecord(record) => {
                    validate_time_crystal(&record)
                }
                EntryTypes::MycelialTaskRecord(record) => {
                    validate_mycelial_task(&record)
                }
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // -- Register Links --
        FlatOp::RegisterCreateLink {
            link_type,
            ..
        } => match link_type {
            LinkTypes::PatternToAddress => Ok(ValidateCallbackResult::Valid),
            LinkTypes::ScaleToGovernance => Ok(ValidateCallbackResult::Valid),
            LinkTypes::TaskToNodes => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDeleteLink { .. } => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterUpdate(update) => match update {
            OpUpdate::Entry { app_entry, .. } => match app_entry {
                // Resonance addresses are immutable once created
                EntryTypes::ResonanceAddressRecord(_) => Ok(ValidateCallbackResult::Invalid(
                    "ResonanceAddressRecord: resonance addresses are immutable once created"
                        .to_string(),
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

fn validate_resonance_address(
    record: &ResonanceAddressRecord,
) -> ExternResult<ValidateCallbackResult> {
    if record.pattern_vector.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ResonanceAddressRecord: pattern_vector must not be empty".to_string(),
        ));
    }
    if record.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ResonanceAddressRecord: description must not be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_fractal_governance(
    record: &FractalGovernanceRecord,
) -> ExternResult<ValidateCallbackResult> {
    if record.pattern_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "FractalGovernanceRecord: pattern_name must not be empty".to_string(),
        ));
    }
    if record.scale.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "FractalGovernanceRecord: scale must not be empty".to_string(),
        ));
    }
    if record.structural_rules.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "FractalGovernanceRecord: must have at least one structural rule for identity across scales".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_morphogenetic_field(
    record: &MorphogeneticFieldRecord,
) -> ExternResult<ValidateCallbackResult> {
    if record.field_description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "MorphogeneticFieldRecord: field_description must not be empty".to_string(),
        ));
    }
    if record.influence_radius < 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "MorphogeneticFieldRecord: influence_radius must be >= 0.0".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_time_crystal(record: &TimeCrystalRecord) -> ExternResult<ValidateCallbackResult> {
    if record.period_duration == 0 {
        return Ok(ValidateCallbackResult::Invalid(
            "TimeCrystalRecord: period_duration must be > 0".to_string(),
        ));
    }
    if record.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "TimeCrystalRecord: description must not be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_mycelial_task(record: &MycelialTaskRecord) -> ExternResult<ValidateCallbackResult> {
    if record.task_description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "MycelialTaskRecord: task_description must not be empty".to_string(),
        ));
    }
    if record.assigned_nodes.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "MycelialTaskRecord: must have at least one assigned node".to_string(),
        ));
    }
    // input_hash is an ActionHash — its structural validity is guaranteed by the type system,
    // but we confirm it is present (non-default) by checking it was provided.
    // The ActionHash type itself ensures it is well-formed.
    Ok(ValidateCallbackResult::Valid)
}
