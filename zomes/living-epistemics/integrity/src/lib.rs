use hdi::prelude::*;
use mycelix_shared::{EpistemicClassification, PresenceProof};

// ============================================================================
// Mycelix v6.0 — Living Epistemics Integrity Zome
// ============================================================================
// Primitives [9]-[12]: Shadow, Held-in-Uncertainty, Silence, Beauty Score

// ---------------------------------------------------------------------------
// Entry Types
// ---------------------------------------------------------------------------

/// [9] ShadowRecord — surfaces a hidden, repressed, or denied aspect of
/// a topic or community pattern for conscious integration.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ShadowRecord {
    pub agent: AgentPubKey,
    pub topic: String,
    pub shadow_description: String,
    pub integration_path: Option<String>,
    pub epistemic: EpistemicClassification,
    pub surfaced_at: Timestamp,
}

/// [10] HeldInUncertaintyClaim — explicitly holds a proposition in
/// uncertainty rather than forcing premature resolution.
/// Must include a reason and an earliest_resolution timestamp in the future.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HeldInUncertaintyClaim {
    pub agent: AgentPubKey,
    pub proposition: String,
    pub reason: String,
    pub earliest_resolution: Timestamp,
    pub resolved: bool,
    pub resolution: Option<String>,
    pub epistemic: EpistemicClassification,
    pub created_at: Timestamp,
}

/// [11] SilenceRecord — records a period of collective silence as an
/// epistemic act. Must include at least one valid PresenceProof.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SilenceRecord {
    pub facilitator: AgentPubKey,
    pub participants: Vec<AgentPubKey>,
    pub duration_seconds: u64,
    pub context: String,
    pub presence_proofs: Vec<PresenceProof>,
    pub epistemic: EpistemicClassification,
    pub started_at: Timestamp,
}

/// [12] BeautyScoreRecord — an aesthetic evaluation of a proposal or
/// artifact along multiple dimensions, each in [0.0, 1.0].
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct BeautyScoreRecord {
    pub scorer: AgentPubKey,
    pub target_hash: ActionHash,
    pub coherence: f64,   // [0.0, 1.0]
    pub elegance: f64,    // [0.0, 1.0]
    pub resonance: f64,   // [0.0, 1.0]
    pub aliveness: f64,   // [0.0, 1.0]
    pub wholeness: f64,   // [0.0, 1.0]
    pub narrative: String,
    pub epistemic: EpistemicClassification,
    pub scored_at: Timestamp,
}

// ---------------------------------------------------------------------------
// Entry / Link Enums
// ---------------------------------------------------------------------------

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    #[entry_type(name = "ShadowRecord")]
    ShadowRecord(ShadowRecord),
    #[entry_type(name = "HeldInUncertaintyClaim")]
    HeldInUncertaintyClaim(HeldInUncertaintyClaim),
    #[entry_type(name = "SilenceRecord")]
    SilenceRecord(SilenceRecord),
    #[entry_type(name = "BeautyScoreRecord")]
    BeautyScoreRecord(BeautyScoreRecord),
}

#[hdk_link_types]
pub enum LinkTypes {
    TopicToShadows,
    ClaimToUncertainty,
    AgentToSilences,
    ProposalToBeautyScores,
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
                    EntryTypes::ShadowRecord(record) => validate_shadow(&record),
                    EntryTypes::HeldInUncertaintyClaim(claim) => {
                        validate_uncertainty_claim(&claim)
                    }
                    EntryTypes::SilenceRecord(record) => validate_silence(&record),
                    EntryTypes::BeautyScoreRecord(record) => validate_beauty_score(&record),
                }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // -- Store Record --
        FlatOp::StoreRecord(store_record) => match store_record {
            OpRecord::CreateEntry { app_entry, .. }
            | OpRecord::UpdateEntry { app_entry, .. } => match app_entry {
                EntryTypes::ShadowRecord(record) => validate_shadow(&record),
                EntryTypes::HeldInUncertaintyClaim(claim) => {
                    validate_uncertainty_claim(&claim)
                }
                EntryTypes::SilenceRecord(record) => validate_silence(&record),
                EntryTypes::BeautyScoreRecord(record) => validate_beauty_score(&record),
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
            LinkTypes::TopicToShadows => Ok(ValidateCallbackResult::Valid),
            LinkTypes::ClaimToUncertainty => Ok(ValidateCallbackResult::Valid),
            LinkTypes::AgentToSilences => Ok(ValidateCallbackResult::Valid),
            LinkTypes::ProposalToBeautyScores => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDeleteLink { .. } => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterUpdate(update) => match update {
            OpUpdate::Entry { app_entry, .. } => match app_entry {
                // Shadow records are immutable once surfaced
                EntryTypes::ShadowRecord(_) => Ok(ValidateCallbackResult::Invalid(
                    "ShadowRecord: shadows cannot be edited once surfaced — they can only be integrated".to_string(),
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

fn validate_shadow(record: &ShadowRecord) -> ExternResult<ValidateCallbackResult> {
    if record.topic.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ShadowRecord: topic must not be empty".to_string(),
        ));
    }
    if record.shadow_description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ShadowRecord: shadow_description must not be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_uncertainty_claim(
    claim: &HeldInUncertaintyClaim,
) -> ExternResult<ValidateCallbackResult> {
    if claim.reason.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "HeldInUncertaintyClaim: must have a reason for holding in uncertainty".to_string(),
        ));
    }
    if claim.proposition.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "HeldInUncertaintyClaim: proposition must not be empty".to_string(),
        ));
    }
    // earliest_resolution must be in the future relative to created_at
    if claim.earliest_resolution <= claim.created_at {
        return Ok(ValidateCallbackResult::Invalid(
            "HeldInUncertaintyClaim: earliest_resolution must be after created_at".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_silence(record: &SilenceRecord) -> ExternResult<ValidateCallbackResult> {
    if record.presence_proofs.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "SilenceRecord: must have at least one valid PresenceProof".to_string(),
        ));
    }
    if record.duration_seconds == 0 {
        return Ok(ValidateCallbackResult::Invalid(
            "SilenceRecord: duration_seconds must be greater than zero".to_string(),
        ));
    }
    if record.participants.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "SilenceRecord: must have at least one participant".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_beauty_score(record: &BeautyScoreRecord) -> ExternResult<ValidateCallbackResult> {
    let scores = [
        ("coherence", record.coherence),
        ("elegance", record.elegance),
        ("resonance", record.resonance),
        ("aliveness", record.aliveness),
        ("wholeness", record.wholeness),
    ];

    for (name, value) in &scores {
        if !(0.0..=1.0).contains(value) {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "BeautyScoreRecord: {} must be in [0.0, 1.0], got {}",
                name, value
            )));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}
