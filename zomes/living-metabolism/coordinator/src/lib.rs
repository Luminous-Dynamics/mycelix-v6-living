use hdk::prelude::*;
use living_metabolism_integrity::*;
use mycelix_shared::{CyclePhase, EpistemicClassification};

// ============================================================================
// Mycelix v6.0 — Living Metabolism Coordinator Zome
// ============================================================================

// ---------------------------------------------------------------------------
// Input Types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateWoundInput {
    pub description: String,
    pub witnesses: Vec<AgentPubKey>,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KenosisInput {
    pub release_description: String,
    pub release_percentage: f64,
    pub beneficiaries: Vec<AgentPubKey>,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompostInput {
    pub source_description: String,
    pub cycle_phase: CyclePhase,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrustInput {
    pub to_agent: AgentPubKey,
    pub score: f64,
    pub context: String,
    pub evidence_hashes: Vec<ActionHash>,
}

// ---------------------------------------------------------------------------
// Extern Functions
// ---------------------------------------------------------------------------

/// Create a new wound record in the Hemostasis phase and link it to the
/// calling agent.
#[hdk_extern]
pub fn create_wound(input: CreateWoundInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let wound = WoundRecord {
        agent: agent.clone(),
        description: input.description,
        phase: WoundPhase::Hemostasis,
        witnesses: input.witnesses,
        restitution_offered: false,
        epistemic: input.epistemic,
        created_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::WoundRecord(wound.clone()))?;

    create_link(
        agent,
        action_hash.clone(),
        LinkTypes::AgentToWounds,
        (),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve created WoundRecord".to_string()
        )))?;

    Ok(record)
}

/// Advance a wound to its next healing phase. Phases only move forward:
/// Hemostasis -> Inflammation -> Proliferation -> Remodeling -> Healed.
#[hdk_extern]
pub fn advance_wound_phase(wound_hash: ActionHash) -> ExternResult<Record> {
    let record = get(wound_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "WoundRecord not found".to_string()
        )))?;

    let mut wound: WoundRecord = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize WoundRecord".to_string()
        )))?;

    let next_phase = match wound.phase {
        WoundPhase::Hemostasis => WoundPhase::Inflammation,
        WoundPhase::Inflammation => WoundPhase::Proliferation,
        WoundPhase::Proliferation => WoundPhase::Remodeling,
        WoundPhase::Remodeling => WoundPhase::Healed,
        WoundPhase::Healed => {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Wound is already fully healed — no further phase advancement".to_string()
            )));
        }
    };

    wound.phase = next_phase;

    let updated_hash = update_entry(wound_hash, EntryTypes::WoundRecord(wound))?;

    let updated_record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve updated WoundRecord".to_string()
        )))?;

    Ok(updated_record)
}

/// Commit an irrevocable kenosis (self-emptying). The release percentage
/// is capped at 20% by integrity validation.
#[hdk_extern]
pub fn commit_kenosis(input: KenosisInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let kenosis = KenosisCommitment {
        agent: agent.clone(),
        release_description: input.release_description,
        release_percentage: input.release_percentage,
        irrevocable: true, // always irrevocable
        beneficiaries: input.beneficiaries,
        epistemic: input.epistemic,
        created_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::KenosisCommitment(kenosis))?;

    create_link(
        agent,
        action_hash.clone(),
        LinkTypes::AgentToKenosis,
        (),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve KenosisCommitment".to_string()
        )))?;

    Ok(record)
}

/// Start a composting process for an outdated pattern or structure.
#[hdk_extern]
pub fn start_composting(input: CompostInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let composting = CompostingRecord {
        agent: agent.clone(),
        source_description: input.source_description,
        decomposition_progress: 0.0,
        nutrients_released: Vec::new(),
        cycle_phase: input.cycle_phase,
        epistemic: input.epistemic,
        created_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::CompostingRecord(composting))?;

    create_link(
        agent,
        action_hash.clone(),
        LinkTypes::AgentToComposting,
        (),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve CompostingRecord".to_string()
        )))?;

    Ok(record)
}

/// Update (or create) a metabolic trust record toward another agent.
#[hdk_extern]
pub fn update_metabolic_trust(input: TrustInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let trust = MetabolicTrustRecord {
        from_agent: agent,
        to_agent: input.to_agent,
        score: input.score,
        context: input.context,
        evidence_hashes: input.evidence_hashes,
        updated_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::MetabolicTrustRecord(trust))?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve MetabolicTrustRecord".to_string()
        )))?;

    Ok(record)
}

/// Retrieve all wound records linked to the given agent.
#[hdk_extern]
pub fn get_wounds_for_agent(agent: AgentPubKey) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::new(agent, LinkTypeFilter::single_type(
            zome_info()?.id,
            LinkType(LinkTypes::AgentToWounds as u8),
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
