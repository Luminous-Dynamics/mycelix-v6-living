use hdk::prelude::*;
use living_structural_integrity::*;
use mycelix_shared::EpistemicClassification;

// ============================================================================
// Mycelix v6.0 — Living Structural Coordinator Zome
// ============================================================================

// ---------------------------------------------------------------------------
// Input Types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddressInput {
    pub pattern_vector: Vec<f64>,
    pub description: String,
    pub referenced_hashes: Vec<ActionHash>,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PatternQuery {
    pub pattern: Vec<f64>,
    pub threshold: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FractalInput {
    pub pattern_name: String,
    pub scale: String,
    pub parent_pattern_hash: Option<ActionHash>,
    pub structural_rules: Vec<String>,
    pub participants: Vec<AgentPubKey>,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReplicateInput {
    pub parent_hash: ActionHash,
    pub child_scale: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimeCrystalInput {
    pub description: String,
    pub period_duration: u64,
    pub phase_offset: u64,
    pub participants: Vec<AgentPubKey>,
    pub epistemic: EpistemicClassification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskInput {
    pub task_description: String,
    pub input_hash: ActionHash,
    pub assigned_nodes: Vec<AgentPubKey>,
    pub epistemic: EpistemicClassification,
}

// ---------------------------------------------------------------------------
// Extern Functions
// ---------------------------------------------------------------------------

/// Create a resonance address — a pattern-based content address for
/// discovery by similarity rather than location.
#[hdk_extern]
pub fn create_resonance_address(input: AddressInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let address = ResonanceAddressRecord {
        creator: agent,
        pattern_vector: input.pattern_vector,
        description: input.description,
        referenced_hashes: input.referenced_hashes,
        epistemic: input.epistemic,
        created_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::ResonanceAddressRecord(address))?;

    // Link from agent to address for later retrieval
    let agent = agent_info()?.agent_initial_pubkey;
    create_link(
        agent,
        action_hash.clone(),
        LinkTypes::PatternToAddress,
        "resonance_address".as_bytes().to_vec(),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve ResonanceAddressRecord".to_string()
        )))?;

    Ok(record)
}

/// Resolve resonance addresses by pattern similarity. Returns all addresses
/// whose cosine similarity to the query pattern exceeds the threshold.
///
/// Uses GetStrategy::Network to query the DHT for consistency across nodes.
/// This is critical for multi-node deployments where links may not have
/// propagated to the local node yet.
#[hdk_extern]
pub fn resolve_by_pattern(query: PatternQuery) -> ExternResult<Vec<Record>> {
    let agent = agent_info()?.agent_initial_pubkey;
    let links = get_links(
        LinkQuery::new(agent, LinkTypeFilter::single_type(
            zome_info()?.id,
            LinkType(LinkTypes::PatternToAddress as u8),
        )),
        GetStrategy::Network, // Network for multi-node consistency
    )?;

    let mut results: Vec<Record> = Vec::new();

    for link in links {
        let target = link
            .target
            .into_action_hash()
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Link target is not an ActionHash".to_string()
            )))?;

        if let Some(record) = get(target, GetOptions::default())? {
            let address: ResonanceAddressRecord = record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Could not deserialize ResonanceAddressRecord".to_string()
                )))?;

            let similarity = cosine_similarity(&query.pattern, &address.pattern_vector);
            if similarity >= query.threshold {
                results.push(record);
            }
        }
    }

    Ok(results)
}

/// Create a fractal governance pattern at a given scale.
#[hdk_extern]
pub fn create_fractal_pattern(input: FractalInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let governance = FractalGovernanceRecord {
        creator: agent.clone(),
        pattern_name: input.pattern_name,
        scale: input.scale.clone(),
        parent_pattern_hash: input.parent_pattern_hash,
        structural_rules: input.structural_rules,
        participants: input.participants,
        epistemic: input.epistemic,
        created_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::FractalGovernanceRecord(governance))?;

    // Link from action to governance for retrieval, store scale in tag
    create_link(
        agent.clone(),
        action_hash.clone(),
        LinkTypes::ScaleToGovernance,
        input.scale.as_bytes().to_vec(),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve FractalGovernanceRecord".to_string()
        )))?;

    Ok(record)
}

/// Replicate a fractal governance pattern at a child scale.
/// The child inherits all structural_rules from the parent to maintain
/// structural identity across scales.
#[hdk_extern]
pub fn replicate_pattern(input: ReplicateInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    // Fetch the parent pattern
    let parent_record = get(input.parent_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Parent FractalGovernanceRecord not found".to_string()
        )))?;

    let parent: FractalGovernanceRecord = parent_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize parent FractalGovernanceRecord".to_string()
        )))?;

    // Create child with inherited structural rules
    let child = FractalGovernanceRecord {
        creator: agent.clone(),
        pattern_name: parent.pattern_name.clone(),
        scale: input.child_scale.clone(),
        parent_pattern_hash: Some(input.parent_hash),
        structural_rules: parent.structural_rules.clone(), // identity across scales
        participants: Vec::new(), // child starts with no participants
        epistemic: parent.epistemic.clone(),
        created_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::FractalGovernanceRecord(child))?;

    create_link(
        agent.clone(),
        action_hash.clone(),
        LinkTypes::ScaleToGovernance,
        input.child_scale.as_bytes().to_vec(),
    )?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve replicated FractalGovernanceRecord".to_string()
        )))?;

    Ok(record)
}

/// Start a time crystal period — a rhythmic temporal structure in the network.
#[hdk_extern]
pub fn start_time_crystal_period(input: TimeCrystalInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let crystal = TimeCrystalRecord {
        creator: agent,
        description: input.description,
        period_duration: input.period_duration,
        phase_offset: input.phase_offset,
        participants: input.participants,
        active: true,
        epistemic: input.epistemic,
        created_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::TimeCrystalRecord(crystal))?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve TimeCrystalRecord".to_string()
        )))?;

    Ok(record)
}

/// Submit a mycelial task — distributed computation across the network.
#[hdk_extern]
pub fn submit_mycelial_task(input: TaskInput) -> ExternResult<Record> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let task = MycelialTaskRecord {
        creator: agent,
        task_description: input.task_description,
        input_hash: input.input_hash,
        assigned_nodes: input.assigned_nodes.clone(),
        status: TaskStatus::Pending,
        result_hash: None,
        epistemic: input.epistemic,
        created_at: Timestamp::from_micros(now.as_micros()),
    };

    let action_hash = create_entry(EntryTypes::MycelialTaskRecord(task))?;

    // Link task to each assigned node
    for node in &input.assigned_nodes {
        create_link(
            action_hash.clone(),
            node.clone(),
            LinkTypes::TaskToNodes,
            (),
        )?;
    }

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not retrieve MycelialTaskRecord".to_string()
        )))?;

    Ok(record)
}

// ---------------------------------------------------------------------------
// Internal Helpers
// ---------------------------------------------------------------------------

/// Compute cosine similarity between two vectors.
/// Returns 0.0 if either vector has zero magnitude or if lengths differ.
fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }

    dot / (mag_a * mag_b)
}
