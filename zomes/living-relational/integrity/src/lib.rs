use hdi::prelude::*;
use mycelix_shared::{CyclePhase, EpistemicClassification};

// ============================================================================
// Mycelix v6.0 — Living Relational Integrity Zome
// ============================================================================
// Primitives [13]-[16]: Entanglement, Attractor Field, Liminal, Inter-Species

// ---------------------------------------------------------------------------
// Entry Types
// ---------------------------------------------------------------------------

/// [13] EntanglementRecord — records a quantum-inspired relational
/// entanglement between agents. Strength must be in [0.0, 1.0].
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EntanglementRecord {
    pub agent_a: AgentPubKey,
    pub agent_b: AgentPubKey,
    pub strength: f64, // [0.0, 1.0]
    pub context: String,
    pub mutual_consent: bool,
    pub epistemic: EpistemicClassification,
    pub formed_at: Timestamp,
}

/// [14] AttractorFieldRecord — describes a relational attractor basin
/// that draws agents toward particular co-creative patterns.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AttractorFieldRecord {
    pub creator: AgentPubKey,
    pub description: String,
    pub field_strength: f64,
    pub attracted_agents: Vec<AgentPubKey>,
    pub cycle_phase: CyclePhase,
    pub epistemic: EpistemicClassification,
    pub created_at: Timestamp,
}

/// Liminal phases — must advance forward only.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum LiminalPhase {
    PreLiminal,
    Liminal,
    PostLiminal,
    Integrated,
}

impl LiminalPhase {
    pub fn ordinal(&self) -> u8 {
        match self {
            LiminalPhase::PreLiminal => 0,
            LiminalPhase::Liminal => 1,
            LiminalPhase::PostLiminal => 2,
            LiminalPhase::Integrated => 3,
        }
    }
}

/// [15] LiminalRecord — tracks an agent's passage through a liminal
/// (threshold) state. Phase transitions must move forward only.
/// While in the Liminal phase, recategorization_blocked must be true.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct LiminalRecord {
    pub agent: AgentPubKey,
    pub description: String,
    pub phase: LiminalPhase,
    pub recategorization_blocked: bool,
    pub witnesses: Vec<AgentPubKey>,
    pub epistemic: EpistemicClassification,
    pub entered_at: Timestamp,
}

/// [16] InterSpeciesRecord — records a relational interaction that
/// crosses species or system-type boundaries.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct InterSpeciesRecord {
    pub initiator: AgentPubKey,
    pub species_type: String,
    pub interaction_description: String,
    pub participants: Vec<AgentPubKey>,
    pub relational_learnings: Vec<String>,
    pub epistemic: EpistemicClassification,
    pub observed_at: Timestamp,
}

// ---------------------------------------------------------------------------
// Entry / Link Enums
// ---------------------------------------------------------------------------

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    #[entry_type(name = "EntanglementRecord")]
    EntanglementRecord(EntanglementRecord),
    #[entry_type(name = "AttractorFieldRecord")]
    AttractorFieldRecord(AttractorFieldRecord),
    #[entry_type(name = "LiminalRecord")]
    LiminalRecord(LiminalRecord),
    #[entry_type(name = "InterSpeciesRecord")]
    InterSpeciesRecord(InterSpeciesRecord),
}

#[hdk_link_types]
pub enum LinkTypes {
    AgentToEntanglements,
    AgentToLiminal,
    SpeciesToParticipants,
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
                    EntryTypes::EntanglementRecord(record) => validate_entanglement(&record),
                    EntryTypes::AttractorFieldRecord(record) => {
                        validate_attractor_field(&record)
                    }
                    EntryTypes::LiminalRecord(record) => validate_liminal(&record),
                    EntryTypes::InterSpeciesRecord(record) => {
                        validate_inter_species(&record)
                    }
                }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // -- Store Record --
        FlatOp::StoreRecord(store_record) => match store_record {
            OpRecord::CreateEntry { app_entry, .. }
            | OpRecord::UpdateEntry { app_entry, .. } => match app_entry {
                EntryTypes::EntanglementRecord(record) => validate_entanglement(&record),
                EntryTypes::AttractorFieldRecord(record) => {
                    validate_attractor_field(&record)
                }
                EntryTypes::LiminalRecord(record) => validate_liminal(&record),
                EntryTypes::InterSpeciesRecord(record) => {
                    validate_inter_species(&record)
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
            LinkTypes::AgentToEntanglements => Ok(ValidateCallbackResult::Valid),
            LinkTypes::AgentToLiminal => Ok(ValidateCallbackResult::Valid),
            LinkTypes::SpeciesToParticipants => Ok(ValidateCallbackResult::Valid),
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

fn validate_entanglement(record: &EntanglementRecord) -> ExternResult<ValidateCallbackResult> {
    if !(0.0..=1.0).contains(&record.strength) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "EntanglementRecord: strength must be in [0.0, 1.0], got {}",
            record.strength
        )));
    }
    if record.agent_a == record.agent_b {
        return Ok(ValidateCallbackResult::Invalid(
            "EntanglementRecord: cannot entangle an agent with themselves".to_string(),
        ));
    }
    if record.context.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "EntanglementRecord: context must not be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_attractor_field(record: &AttractorFieldRecord) -> ExternResult<ValidateCallbackResult> {
    if record.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "AttractorFieldRecord: description must not be empty".to_string(),
        ));
    }
    if record.field_strength < 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "AttractorFieldRecord: field_strength must be >= 0.0".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_liminal(record: &LiminalRecord) -> ExternResult<ValidateCallbackResult> {
    // While in the Liminal phase, recategorization_blocked must be true
    if record.phase == LiminalPhase::Liminal && !record.recategorization_blocked {
        return Ok(ValidateCallbackResult::Invalid(
            "LiminalRecord: recategorization_blocked must be true while in Liminal phase"
                .to_string(),
        ));
    }
    if record.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "LiminalRecord: description must not be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_inter_species(record: &InterSpeciesRecord) -> ExternResult<ValidateCallbackResult> {
    if record.species_type.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "InterSpeciesRecord: species_type must not be empty".to_string(),
        ));
    }
    if record.interaction_description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "InterSpeciesRecord: interaction_description must not be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
