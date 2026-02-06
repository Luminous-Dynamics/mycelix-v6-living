use hdk::prelude::*;
use living_epistemics_integrity::*;
use mycelix_shared::{EpistemicClassification, PresenceProof};

// ============================================================================
// Mycelix v6.0 — Living Epistemics Coordinator Zome
// ============================================================================

// ---------------------------------------------------------------------------
// Input Types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShadowInput {
    pub topic: String,
    pub shadow_description: String,
    pub integration_path: Option<String>,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UncertaintyInput {
    pub proposition: String,
    pub reason: String,
    pub earliest_resolution: Timestamp,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SilenceInput {
    pub participants: Vec<AgentPubKey>,
    pub duration_seconds: u64,
    pub context: String,
    pub presence_proofs: Vec<PresenceProof>,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BeautyInput {
    pub target_hash: ActionHash,
    pub coherence: f64,
    pub elegance: f64,
    pub resonance: f64,
    pub aliveness: f64,
    pub wholeness: f64,
    pub narrative: String,
    pub epistemic: EpistemicClassification,
}

// ---------------------------------------------------------------------------
// Extern Functions
// ---------------------------------------------------------------------------

/// Surface a shadow — bring a hidden or denied aspect into collective awareness.
/// Shadow records are immutable once created (enforced by integrity validation).
#[hdk_extern]
pub fn surface_shadow(input: ShadowInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let shadow = ShadowRecord {
        agent: agent.clone(),
        topic: input.topic.clone(),
        shadow_description: input.shadow_description,
        integration_path: input.integration_path,
        epistemic: input.epistemic,
        surfaced_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::ShadowRecord(shadow))?;

    // Link from the agent to the shadow for retrieval (using agent as anchor)
    create_link(
        agent.clone(),
        action_hash.clone(),
        LinkTypes::TopicToShadows,
        input.topic.as_bytes().to_vec(),  // Store topic in link tag
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve ShadowRecord".to_string()
        )))?;

    Ok(record)
}

/// Hold a proposition in uncertainty — resist premature closure.
/// The earliest_resolution timestamp must be in the future.
#[hdk_extern]
pub fn hold_in_uncertainty(input: UncertaintyInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let claim = HeldInUncertaintyClaim {
        agent,
        proposition: input.proposition,
        reason: input.reason,
        earliest_resolution: input.earliest_resolution,
        resolved: false,
        resolution: None,
        epistemic: input.epistemic,
        created_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::HeldInUncertaintyClaim(claim))?;

    create_link(
        action_hash.clone(),
        action_hash.clone(),
        LinkTypes::ClaimToUncertainty,
        (),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve HeldInUncertaintyClaim".to_string()
        )))?;

    Ok(record)
}

/// Release a proposition from uncertainty with a resolution.
/// Only possible after the earliest_resolution timestamp has passed.
#[hdk_extern]
pub fn release_from_uncertainty(input: ReleaseInput) -> ExternResult<Record> {
    let record = get(input.claim_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "HeldInUncertaintyClaim not found".to_string()
        )))?;

    let mut claim: HeldInUncertaintyClaim = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize HeldInUncertaintyClaim".to_string()
        )))?;

    if claim.resolved {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "This claim has already been resolved".to_string()
        )));
    }

    // Check that we have passed the earliest resolution time
    let now = sys_time()?;
    let now_ts = Timestamp::from_micros(now.as_micros());
    if now_ts < claim.earliest_resolution {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Cannot release from uncertainty before earliest_resolution timestamp".to_string()
        )));
    }

    claim.resolved = true;
    claim.resolution = Some(input.resolution);

    let updated_hash = update_entry(
        input.claim_hash,
        EntryTypes::HeldInUncertaintyClaim(claim),
    )?;

    let updated_record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve updated HeldInUncertaintyClaim".to_string()
        )))?;

    Ok(updated_record)
}

/// Record a period of collective silence as an epistemic practice.
/// Requires at least one PresenceProof.
#[hdk_extern]
pub fn record_silence(input: SilenceInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let silence = SilenceRecord {
        facilitator: agent.clone(),
        participants: input.participants,
        duration_seconds: input.duration_seconds,
        context: input.context,
        presence_proofs: input.presence_proofs,
        epistemic: input.epistemic,
        started_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::SilenceRecord(silence))?;

    create_link(
        agent,
        action_hash.clone(),
        LinkTypes::AgentToSilences,
        (),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve SilenceRecord".to_string()
        )))?;

    Ok(record)
}

/// Submit an aesthetic beauty score for a proposal or artifact.
/// All component scores must be in [0.0, 1.0].
#[hdk_extern]
pub fn submit_beauty_score(input: BeautyInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let beauty = BeautyScoreRecord {
        scorer: agent,
        target_hash: input.target_hash.clone(),
        coherence: input.coherence,
        elegance: input.elegance,
        resonance: input.resonance,
        aliveness: input.aliveness,
        wholeness: input.wholeness,
        narrative: input.narrative,
        epistemic: input.epistemic,
        scored_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::BeautyScoreRecord(beauty))?;

    create_link(
        input.target_hash,
        action_hash.clone(),
        LinkTypes::ProposalToBeautyScores,
        (),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve BeautyScoreRecord".to_string()
        )))?;

    Ok(record)
}

/// Retrieve all beauty scores linked to a given proposal.
#[hdk_extern]
pub fn get_beauty_scores(proposal_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::new(proposal_hash, LinkTypeFilter::single_type(
            zome_info()?.id,
            LinkType(LinkTypes::ProposalToBeautyScores as u8),
        )),
        GetStrategy::Local,
    )?;

    let mut records: Vec<Record> = Vec::new();
    for link in links {
        let target = link
            .target
            .into_action_hash()
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Link target is not an ActionHash".to_string()
            )))?;

        if let Some(record) = get(target, GetOptions::default())? {
            records.push(record);
        }
    }

    Ok(records)
}

// ---------------------------------------------------------------------------
// Helper Structs
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReleaseInput {
    pub claim_hash: ActionHash,
    pub resolution: String,
}
