use hdk::prelude::*;
use living_relational_integrity::*;
use mycelix_shared::{CyclePhase, EpistemicClassification};

// ============================================================================
// Mycelix v6.0 — Living Relational Coordinator Zome
// ============================================================================

// ---------------------------------------------------------------------------
// Input Types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CoCreationInput {
    pub description: String,
    pub attracted_agents: Vec<AgentPubKey>,
    pub cycle_phase: CyclePhase,
    pub field_strength: f64,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EntanglementInput {
    pub partner: AgentPubKey,
    pub strength: f64,
    pub context: String,
    pub mutual_consent: bool,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiminalInput {
    pub description: String,
    pub witnesses: Vec<AgentPubKey>,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterSpeciesInput {
    pub species_type: String,
    pub interaction_description: String,
    pub participants: Vec<AgentPubKey>,
    pub relational_learnings: Vec<String>,
    pub epistemic: EpistemicClassification,
}

// ---------------------------------------------------------------------------
// Extern Functions
// ---------------------------------------------------------------------------

/// Record a co-creation event by creating an attractor field record.
#[hdk_extern]
pub fn record_co_creation(input: CoCreationInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let attractor = AttractorFieldRecord {
        creator: agent,
        description: input.description,
        field_strength: input.field_strength,
        attracted_agents: input.attracted_agents,
        cycle_phase: input.cycle_phase,
        epistemic: input.epistemic,
        created_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::AttractorFieldRecord(attractor))?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve AttractorFieldRecord".to_string()
        )))?;

    Ok(record)
}

/// Form a relational entanglement between the calling agent and a partner.
/// Both agents are linked to the entanglement record.
#[hdk_extern]
pub fn form_entanglement(input: EntanglementInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let entanglement = EntanglementRecord {
        agent_a: agent.clone(),
        agent_b: input.partner.clone(),
        strength: input.strength,
        context: input.context,
        mutual_consent: input.mutual_consent,
        epistemic: input.epistemic,
        formed_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::EntanglementRecord(entanglement))?;

    // Link both agents to the entanglement
    create_link(
        agent,
        action_hash.clone(),
        LinkTypes::AgentToEntanglements,
        (),
    )?;
    create_link(
        input.partner,
        action_hash.clone(),
        LinkTypes::AgentToEntanglements,
        (),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve EntanglementRecord".to_string()
        )))?;

    Ok(record)
}

/// Enter a liminal (threshold) state. Begins in PreLiminal phase.
#[hdk_extern]
pub fn enter_liminal_state(input: LiminalInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let liminal = LiminalRecord {
        agent: agent.clone(),
        description: input.description,
        phase: LiminalPhase::PreLiminal,
        recategorization_blocked: false, // not yet in Liminal phase
        witnesses: input.witnesses,
        epistemic: input.epistemic,
        entered_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::LiminalRecord(liminal))?;

    create_link(
        agent,
        action_hash.clone(),
        LinkTypes::AgentToLiminal,
        (),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve LiminalRecord".to_string()
        )))?;

    Ok(record)
}

/// Advance a liminal record to the next phase.
/// PreLiminal -> Liminal -> PostLiminal -> Integrated.
/// Phase transitions only move forward.
/// When entering Liminal phase, recategorization_blocked is set to true.
/// When leaving Liminal phase, recategorization_blocked is set to false.
#[hdk_extern]
pub fn advance_liminal_phase(record_hash: ActionHash) -> ExternResult<Record> {
    let record = get(record_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "LiminalRecord not found".to_string()
        )))?;

    let mut liminal: LiminalRecord = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize LiminalRecord".to_string()
        )))?;

    let (next_phase, blocked) = match liminal.phase {
        LiminalPhase::PreLiminal => (LiminalPhase::Liminal, true),
        LiminalPhase::Liminal => (LiminalPhase::PostLiminal, false),
        LiminalPhase::PostLiminal => (LiminalPhase::Integrated, false),
        LiminalPhase::Integrated => {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Liminal state is already fully integrated — no further advancement".to_string()
            )));
        }
    };

    liminal.phase = next_phase;
    liminal.recategorization_blocked = blocked;

    let updated_hash = update_entry(record_hash, EntryTypes::LiminalRecord(liminal))?;

    let updated_record = get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve updated LiminalRecord".to_string()
        )))?;

    Ok(updated_record)
}

/// Register an inter-species relational interaction.
#[hdk_extern]
pub fn register_inter_species(input: InterSpeciesInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let inter_species = InterSpeciesRecord {
        initiator: agent,
        species_type: input.species_type.clone(),
        interaction_description: input.interaction_description,
        participants: input.participants.clone(),
        relational_learnings: input.relational_learnings,
        epistemic: input.epistemic,
        observed_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::InterSpeciesRecord(inter_species))?;

    // Link from action hash to participants with species type in tag
    for participant in &input.participants {
        create_link(
            action_hash.clone(),
            participant.clone(),
            LinkTypes::SpeciesToParticipants,
            input.species_type.as_bytes().to_vec(),  // Store species in link tag
        )?;
    }

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve InterSpeciesRecord".to_string()
        )))?;

    Ok(record)
}

/// Retrieve all entanglement records linked to the given agent.
///
/// Uses GetStrategy::Network to query the DHT for consistency across nodes.
/// This is critical for multi-node deployments where links may not have
/// propagated to the local node yet.
#[hdk_extern]
pub fn get_entanglements_for_agent(agent: AgentPubKey) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::new(agent, LinkTypeFilter::single_type(
            zome_info()?.id,
            LinkType(LinkTypes::AgentToEntanglements as u8),
        )),
        GetStrategy::Network, // Network for multi-node consistency
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

/// Retrieve all liminal records linked to the given agent.
///
/// Uses GetStrategy::Network to query the DHT for consistency across nodes.
/// This is critical for multi-node deployments where links may not have
/// propagated to the local node yet.
#[hdk_extern]
pub fn get_liminal_states_for_agent(agent: AgentPubKey) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::new(agent, LinkTypeFilter::single_type(
            zome_info()?.id,
            LinkType(LinkTypes::AgentToLiminal as u8),
        )),
        GetStrategy::Network, // Network for multi-node consistency
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
