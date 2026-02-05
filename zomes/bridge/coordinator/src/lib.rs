use hdk::prelude::*;
use bridge_integrity::*;
use mycelix_shared::{CyclePhase, Did};

// ============================================================================
// Mycelix v6.0 Bridge Zome — Coordinator
// ============================================================================
// Cross-DNA call functions for integrating living-protocol with mycelix-property.
// See INTEGRATION.md for the full integration architecture.

// ============================================================================
// MATL Score Integration
// ============================================================================

/// Fetch MATL score from mycelix-property DNA.
///
/// This crosses DNA boundary to retrieve the v5.2 MATL score which is then
/// used as an input to the v6.0 MetabolicTrustEngine.
#[hdk_extern]
pub fn fetch_matl_score(request: MatlScoreRequest) -> ExternResult<MatlScoreResponse> {
    // Record the bridge call for auditability
    let call_record = record_bridge_call(
        LIVING_PROTOCOL_DNA_NAME,
        PROPERTY_DNA_NAME,
        "get_matl_score",
        true,
        None,
    )?;

    // In production, this would be:
    // call_remote(
    //     None, // Use the bridge capability
    //     PROPERTY_DNA_NAME.into(),
    //     ZomeName::from("governance"),
    //     FunctionName::from("get_matl_score"),
    //     None,
    //     request.clone(),
    // )?

    // For now, return a stub response indicating the integration point
    let now = sys_time()?;
    Ok(MatlScoreResponse {
        agent: request.agent,
        matl_score: 0.5, // Placeholder
        throughput_component: 0.0,
        resilience_component: 0.0,
        composting_component: 0.0,
        timestamp: now,
    })
}

/// Cache a MATL score locally to reduce cross-DNA calls.
#[hdk_extern]
pub fn cache_matl_score(response: MatlScoreResponse) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let cache_duration_micros: i64 = 3600 * 1_000_000; // 1 hour
    let expires_at = Timestamp::from_micros(now.as_micros() + cache_duration_micros);

    let cached = CachedMatlScore {
        agent: response.agent,
        matl_score: response.matl_score,
        throughput_component: response.throughput_component,
        resilience_component: response.resilience_component,
        composting_component: response.composting_component,
        fetched_at: now,
        expires_at,
    };

    create_entry(BridgeEntryTypes::CachedMatlScore(cached))
}

// ============================================================================
// Slash -> Wound Migration
// ============================================================================

/// Intercept a slash event and convert it to a wound healing process.
///
/// This is the core migration function that replaces punitive slashing with
/// the restorative wound healing model.
#[hdk_extern]
pub fn intercept_slash(event: SlashInterceptEvent) -> ExternResult<WoundFromSlash> {
    let now = sys_time()?;
    let severity = WoundSeverity::from_slash_percentage(event.slash_percentage);

    let wound = WoundFromSlash {
        agent: event.offender.clone(),
        severity: severity.clone(),
        source_slash_action: event.original_action_hash.clone(),
        escrow_address: None, // Set by WoundEscrow.sol deployment
        healing_started: now,
    };

    // Record the bridge call
    let _call_record = record_bridge_call(
        PROPERTY_DNA_NAME,
        LIVING_PROTOCOL_DNA_NAME,
        "intercept_slash",
        true,
        None,
    )?;

    // In production, this would:
    // 1. Create a WoundRecord in the living_metabolism zome
    // 2. Deploy/interact with WoundEscrow.sol for the escrowed funds
    // 3. Start the healing arc timer

    Ok(wound)
}

/// Check if slashing should be intercepted or allowed through.
///
/// During the migration period, a feature flag controls whether slashes
/// are converted to wounds or processed normally.
#[hdk_extern]
pub fn should_intercept_slash(_: ()) -> ExternResult<bool> {
    // In production, this would check the FeatureFlags from living-core
    // For now, default to intercepting (new wound healing model)
    Ok(true)
}

// ============================================================================
// K-Vector Integration
// ============================================================================

/// Fetch K-Vector snapshot from mycelix-property for temporal analysis.
///
/// The v5.2 K-Vector is a 5-dimensional snapshot. The v6.0 TemporalKVectorService
/// wraps these snapshots to compute derivatives, velocity, and predictions.
#[hdk_extern]
pub fn fetch_k_vector_snapshot(request: KVectorSnapshotRequest) -> ExternResult<KVectorSnapshotResponse> {
    let _call_record = record_bridge_call(
        LIVING_PROTOCOL_DNA_NAME,
        PROPERTY_DNA_NAME,
        "get_k_vector",
        true,
        None,
    )?;

    // Placeholder response - in production uses call_remote
    let now = sys_time()?;
    Ok(KVectorSnapshotResponse {
        agent: request.agent,
        dimensions: KVectorDimensions {
            stability: 0.5,
            adaptability: 0.5,
            integrity: 0.5,
            connectivity: 0.5,
            emergence: 0.5,
        },
        timestamp: now,
        action_hash: ActionHash::from_raw_36(vec![0u8; 36]),
    })
}

// ============================================================================
// Beauty Score Integration
// ============================================================================

/// Attach a beauty score to a governance proposal.
///
/// During the Beauty phase, proposals are scored on 5 aesthetic dimensions.
/// The score is attached as metadata to the v5.2 governance record.
#[hdk_extern]
pub fn attach_beauty_score(request: AttachBeautyScoreRequest) -> ExternResult<ActionHash> {
    let _call_record = record_bridge_call(
        LIVING_PROTOCOL_DNA_NAME,
        PROPERTY_DNA_NAME,
        "attach_proposal_metadata",
        true,
        None,
    )?;

    // In production, this would call into mycelix-property::governance
    // to attach the beauty score as metadata on the proposal

    // For now, return the original proposal hash
    Ok(request.proposal_action_hash)
}

// ============================================================================
// DID Resolution
// ============================================================================

/// Resolve a DID to agent information via mycelix-property agent registry.
#[hdk_extern]
pub fn resolve_did(request: DidResolveRequest) -> ExternResult<DidResolveResponse> {
    let _call_record = record_bridge_call(
        LIVING_PROTOCOL_DNA_NAME,
        PROPERTY_DNA_NAME,
        "resolve_did",
        true,
        None,
    )?;

    // Placeholder - in production uses call_remote to agent_registry
    Ok(DidResolveResponse {
        did: request.did,
        agent_pub_key: agent_info()?.agent_initial_pubkey,
        display_name: None,
        verified: false,
    })
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Record a bridge call for auditability.
fn record_bridge_call(
    source_dna: &str,
    target_dna: &str,
    function_name: &str,
    success: bool,
    error_message: Option<String>,
) -> ExternResult<ActionHash> {
    let agent = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let record = BridgeCallRecord {
        source_dna: source_dna.to_string(),
        target_dna: target_dna.to_string(),
        function_name: function_name.to_string(),
        caller: agent,
        timestamp: now,
        success,
        error_message,
    };

    create_entry(BridgeEntryTypes::BridgeCallRecord(record))
}

/// Get recent bridge calls for monitoring.
#[hdk_extern]
pub fn get_recent_bridge_calls(_: ()) -> ExternResult<Vec<BridgeCallRecord>> {
    // In production, this would query the local DHT for recent BridgeCallRecord entries
    Ok(vec![])
}

// ============================================================================
// Migration Status
// ============================================================================

/// Status of the v5.2 -> v6.0 migration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MigrationStatus {
    pub slash_interception_enabled: bool,
    pub wounds_created: u64,
    pub wounds_healed: u64,
    pub pending_escrows: u64,
}

/// Get the current migration status.
#[hdk_extern]
pub fn get_migration_status(_: ()) -> ExternResult<MigrationStatus> {
    // Placeholder - in production aggregates from wound healing records
    Ok(MigrationStatus {
        slash_interception_enabled: true,
        wounds_created: 0,
        wounds_healed: 0,
        pending_escrows: 0,
    })
}
