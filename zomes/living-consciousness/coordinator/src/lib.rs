use hdk::prelude::*;
use living_consciousness_integrity::*;
use mycelix_shared::EpistemicClassification;

// ============================================================================
// Mycelix v6.0 — Living Consciousness Coordinator Zome
// ============================================================================

// ---------------------------------------------------------------------------
// Input Types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KVectorInput {
    pub dimensions: Vec<f64>,
    pub context: String,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterferenceInput {
    pub participants: Vec<AgentPubKey>,
    pub interference_type: InterferenceType,
    pub amplitude: f64,
    pub description: String,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DreamInput {
    pub title: String,
    pub description: String,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PhiInput {
    pub measured_agents: Vec<AgentPubKey>,
    pub phi: f64,
    pub measurement_method: String,
    pub contributing_factors: Vec<String>,
    pub epistemic: EpistemicClassification,
}

// ---------------------------------------------------------------------------
// Extern Functions
// ---------------------------------------------------------------------------

/// Submit a consciousness k-vector snapshot with 8 dimensions.
#[hdk_extern]
pub fn submit_k_vector_snapshot(input: KVectorInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let snapshot = TemporalKVectorSnapshot {
        agent: agent.clone(),
        dimensions: input.dimensions,
        context: input.context,
        epistemic: input.epistemic,
        captured_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::TemporalKVectorSnapshot(snapshot))?;

    create_link(
        agent,
        action_hash.clone(),
        LinkTypes::AgentToKVectorHistory,
        (),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve TemporalKVectorSnapshot".to_string()
        )))?;

    Ok(record)
}

/// Record an observed interference pattern between consciousness fields.
#[hdk_extern]
pub fn record_field_interference(input: InterferenceInput) -> ExternResult<Record> {
    let now = sys_time()?;

    let record_entry = FieldInterferenceRecord {
        participants: input.participants,
        interference_type: input.interference_type,
        amplitude: input.amplitude,
        description: input.description,
        epistemic: input.epistemic,
        observed_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::FieldInterferenceRecord(record_entry))?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve FieldInterferenceRecord".to_string()
        )))?;

    Ok(record)
}

/// Submit a dream proposal. Financial operations are automatically set to
/// false and enforced by integrity validation.
#[hdk_extern]
pub fn submit_dream_proposal(input: DreamInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let proposal = DreamProposal {
        proposer: agent,
        title: input.title,
        description: input.description,
        financial_operations: false, // hardcoded — dreams never touch finance
        confirmations: Vec::new(),
        rejection_count: 0,
        epistemic: input.epistemic,
        proposed_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::DreamProposal(proposal))?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve DreamProposal".to_string()
        )))?;

    Ok(record)
}

/// Confirm or reject a dream proposal. Returns true if the proposal has
/// now reached a confirmation threshold (>= 3 unique confirmations).
#[hdk_extern]
pub fn confirm_dream_proposal(
    input: ConfirmDreamInput,
) -> ExternResult<bool> {
    let agent = agent_info()?.agent_initial_pubkey;

    let record = get(input.proposal_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "DreamProposal not found".to_string()
        )))?;

    let mut proposal: DreamProposal = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize DreamProposal".to_string()
        )))?;

    if input.vote {
        if !proposal.confirmations.contains(&agent) {
            proposal.confirmations.push(agent.clone());
        }
    } else {
        proposal.rejection_count += 1;
    }

    update_entry(input.proposal_hash.clone(), EntryTypes::DreamProposal(proposal.clone()))?;

    // Link the confirming agent to the proposal
    if input.vote {
        create_link(
            input.proposal_hash,
            agent,
            LinkTypes::DreamToConfirmation,
            (),
        )?;
    }

    // Threshold: >= 3 unique confirmations
    let reached_threshold = proposal.confirmations.len() >= 3;
    Ok(reached_threshold)
}

/// Record a network phi (integrated information) measurement.
#[hdk_extern]
pub fn record_network_phi(input: PhiInput) -> ExternResult<Record> {
    let now = sys_time()?;

    let measurement = NetworkPhiMeasurement {
        measured_agents: input.measured_agents.clone(),
        phi: input.phi,
        measurement_method: input.measurement_method,
        contributing_factors: input.contributing_factors,
        epistemic: input.epistemic,
        measured_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::NetworkPhiMeasurement(measurement))?;

    // Link each measured agent to the phi measurement
    for agent in &input.measured_agents {
        create_link(
            action_hash.clone(),
            agent.clone(),
            LinkTypes::PhiToAgents,
            (),
        )?;
    }

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve NetworkPhiMeasurement".to_string()
        )))?;

    Ok(record)
}

/// Retrieve all k-vector snapshots linked to the given agent.
///
/// Uses GetStrategy::Network to query the DHT for consistency across nodes.
/// This is critical for multi-node deployments where links may not have
/// propagated to the local node yet.
#[hdk_extern]
pub fn get_k_vector_history(agent: AgentPubKey) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::new(agent, LinkTypeFilter::single_type(
            zome_info()?.id,
            LinkType(LinkTypes::AgentToKVectorHistory as u8),
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

/// Retrieve all confirmations linked to a dream proposal.
///
/// Uses GetStrategy::Network to query the DHT for consistency across nodes.
/// This is critical for multi-node deployments where links may not have
/// propagated to the local node yet.
#[hdk_extern]
pub fn get_dream_confirmations(proposal_hash: ActionHash) -> ExternResult<Vec<AgentPubKey>> {
    let links = get_links(
        LinkQuery::new(proposal_hash, LinkTypeFilter::single_type(
            zome_info()?.id,
            LinkType(LinkTypes::DreamToConfirmation as u8),
        )),
        GetStrategy::Network, // Network for multi-node consistency
    )?;

    let mut agents: Vec<AgentPubKey> = Vec::new();
    for link in links {
        if let Some(agent) = link.target.into_agent_pub_key() {
            agents.push(agent);
        }
    }

    Ok(agents)
}

// ---------------------------------------------------------------------------
// Helper Structs
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfirmDreamInput {
    pub proposal_hash: ActionHash,
    pub vote: bool,
}
