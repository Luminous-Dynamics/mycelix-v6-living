//! End-to-End Cycle Simulation Tests
//!
//! Tests a full 28-day metabolism cycle with realistic data, simulating
//! progression through all 9 phases with phase-specific operations and
//! cross-primitive interactions.
//!
//! ## Test Coverage
//!
//! 1. Full cycle progression through all 9 phases
//! 2. Phase-specific operations (composting, kenosis, etc.)
//! 3. Cross-primitive interactions (shadow -> composting)
//! 4. Multi-cycle simulation with state persistence
//! 5. Realistic multi-agent scenarios

use std::sync::Arc;

use chrono::Utc;

use cycle_engine::phase_handlers::{
    BeautyPhaseHandler, CoCreationPhaseHandler, CompostingPhaseHandler,
    EmergentPersonhoodPhaseHandler, ErosPhaseHandler, KenosisPhaseHandler, LiminalPhaseHandler,
    NegativeCapabilityPhaseHandler, PhaseHandler, ShadowPhaseHandler,
};
use cycle_engine::{CycleEngineBuilder, MetabolismCycleEngine};
use living_core::{
    CompostableEntity, CyclePhase, CycleState, EntanglementConfig, EpistemicClassification,
    EpistemicTier, EventBus, FeatureFlags, InMemoryEventBus, KenosisConfig, LiminalEntityType,
    LivingProtocolConfig, LivingProtocolEvent, MaterialityTier, NegativeCapabilityConfig,
    NormativeTier, ShadowConfig,
};
use metabolism::composting::CompostingReason;

// =========================================================================
// Test Helpers
// =========================================================================

/// Create an engine configured for simulated time testing.
fn create_test_engine() -> MetabolismCycleEngine {
    CycleEngineBuilder::new()
        .with_simulated_time(86400.0) // 1 second = 1 day
        .build()
}

/// Create a test engine with custom config.
fn create_engine_with_config(config: LivingProtocolConfig) -> MetabolismCycleEngine {
    CycleEngineBuilder::new()
        .with_config(config)
        .with_simulated_time(86400.0)
        .build()
}

/// Standard epistemic classification for test nutrients.
fn test_classification() -> EpistemicClassification {
    EpistemicClassification {
        e: EpistemicTier::Testimonial,
        n: NormativeTier::Communal,
        m: MaterialityTier::Persistent,
    }
}

/// Make a CycleState for a given phase.
fn make_state(cycle: u64, phase: CyclePhase, phase_day: u32) -> CycleState {
    CycleState {
        cycle_number: cycle,
        current_phase: phase,
        phase_started: Utc::now(),
        cycle_started: Utc::now(),
        phase_day,
    }
}

// =========================================================================
// Test: Full 28-Day Cycle Simulation
// =========================================================================

#[test]
fn test_full_28_day_cycle_simulation() {
    let mut engine = create_test_engine();
    engine.start().unwrap();

    // All phases in order
    let phases = [
        (CyclePhase::Shadow, 2),
        (CyclePhase::Composting, 5),
        (CyclePhase::Liminal, 3),
        (CyclePhase::NegativeCapability, 3),
        (CyclePhase::Eros, 4),
        (CyclePhase::CoCreation, 7),
        (CyclePhase::Beauty, 2),
        (CyclePhase::EmergentPersonhood, 1),
        (CyclePhase::Kenosis, 1),
    ];

    // Start in Shadow phase
    assert_eq!(engine.current_phase(), CyclePhase::Shadow);
    assert_eq!(engine.cycle_number(), 1);

    // Simulate ticks within Shadow phase
    for _ in 0..3 {
        let tick_events = engine.tick().unwrap();
        // Ticks should succeed (may or may not produce events)
        assert!(
            tick_events.len() <= 10,
            "Tick should not produce excessive events"
        );
    }

    // Progress through all phases
    let mut total_days = 2; // Start with Shadow's 2 days
    for (expected_next, duration) in phases.iter().skip(1) {
        let transition_events = engine.force_transition().unwrap();

        // Verify we're in the expected phase
        assert_eq!(
            engine.current_phase(),
            *expected_next,
            "Should be in {:?} phase",
            expected_next
        );

        // Each transition should produce events
        assert!(
            !transition_events.is_empty(),
            "Transition to {:?} should produce events",
            expected_next
        );

        // Verify transition event is present
        let has_transition_event = transition_events
            .iter()
            .any(|e| matches!(e, LivingProtocolEvent::PhaseTransitioned(_)));
        assert!(
            has_transition_event,
            "Transition to {:?} should include PhaseTransitioned event",
            expected_next
        );

        // Simulate ticks within this phase
        for _ in 0..*duration {
            let _tick_events = engine.tick().unwrap();
        }

        total_days += duration;
    }

    // Complete cycle: transition back to Shadow
    let final_transition = engine.force_transition().unwrap();
    assert_eq!(engine.current_phase(), CyclePhase::Shadow);
    assert_eq!(engine.cycle_number(), 2);
    assert_eq!(total_days, 28); // Full 28-day cycle

    // Should have cycle started event for cycle 2
    let has_cycle_started = final_transition
        .iter()
        .any(|e| matches!(e, LivingProtocolEvent::CycleStarted(ev) if ev.cycle_number == 2));
    assert!(
        has_cycle_started,
        "Should have CycleStarted event for cycle 2"
    );

    // Verify transition history
    assert_eq!(engine.transition_history().len(), 9);
}

// =========================================================================
// Test: Shadow Phase Operations
// =========================================================================

#[test]
fn test_shadow_phase_surfaces_suppressed_content() {
    let mut handler = ShadowPhaseHandler::new(0.3, ShadowConfig::default());

    // Record suppressed content
    handler.engine_mut().record_suppression(
        "controversial-proposal",
        "minority view disagreement",
        0.8,   // high-rep suppressor
        0.15,  // low-rep author (should be prioritized)
        false, // not gate1 protected
    );

    handler
        .engine_mut()
        .record_suppression("standard-content", "off-topic", 0.7, 0.5, false);

    let state = make_state(1, CyclePhase::Shadow, 0);

    // Enter phase
    handler.on_enter(&state).unwrap();

    // Tick should surface the suppressed content
    let events = handler.on_tick(&state).unwrap();

    // At least one shadow surfaced event
    assert!(
        !events.is_empty(),
        "Shadow phase should surface suppressed content"
    );

    let shadow_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, LivingProtocolEvent::ShadowSurfaced(_)))
        .collect();
    assert!(
        !shadow_events.is_empty(),
        "Should have ShadowSurfaced events"
    );

    // Verify low-rep content was prioritized
    if let LivingProtocolEvent::ShadowSurfaced(event) = &shadow_events[0] {
        assert!(
            event.shadow.low_rep_dissent,
            "Low-rep dissent should be surfaced first"
        );
    }

    // Exit phase
    handler.on_exit(&state).unwrap();

    // Verify metrics
    let metrics = handler.collect_metrics();
    assert!(metrics["surfaced_count"].as_u64().unwrap() > 0);
}

// =========================================================================
// Test: Composting Phase Operations
// =========================================================================

#[test]
fn test_composting_phase_decomposes_failed_entities() {
    let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
    let mut handler =
        CompostingPhaseHandler::new(living_core::CompostingConfig::default(), event_bus);

    let state = make_state(1, CyclePhase::Composting, 0);

    // Enter composting phase
    handler.on_enter(&state).unwrap();

    // Start composting a failed proposal
    let record = handler
        .engine_mut()
        .start_composting(
            CompostableEntity::FailedProposal,
            "proposal-failed-quorum".to_string(),
            CompostingReason::ProposalFailed {
                vote_count: 5,
                required: 20,
            },
        )
        .unwrap();

    // Extract nutrients (learnings)
    handler
        .engine_mut()
        .extract_nutrient(
            &record.id,
            "Proposals need clearer success metrics before voting begins".to_string(),
            test_classification(),
        )
        .unwrap();

    handler
        .engine_mut()
        .extract_nutrient(
            &record.id,
            "Consider phased rollout for controversial changes".to_string(),
            test_classification(),
        )
        .unwrap();

    // Tick to update metrics
    handler.on_tick(&state).unwrap();

    // Complete composting
    let nutrients = handler
        .engine_mut()
        .complete_composting(&record.id)
        .unwrap();
    assert_eq!(nutrients.len(), 2);

    // Verify decomposition progress
    let completed_record = handler.engine().get_record(&record.id).unwrap();
    assert_eq!(completed_record.decomposition_progress, 1.0);

    // Exit phase
    handler.on_exit(&state).unwrap();

    // Verify metrics
    let metrics = handler.collect_metrics();
    assert_eq!(metrics["active_composting"].as_u64().unwrap(), 0);
}

// =========================================================================
// Test: Liminal Phase Operations
// =========================================================================

#[test]
fn test_liminal_phase_blocks_recategorization() {
    let mut handler = LiminalPhaseHandler::new();
    let state = make_state(1, CyclePhase::Liminal, 0);

    // Enter liminal phase
    handler.on_enter(&state).unwrap();

    // Enter an entity into liminal transition
    handler.engine_mut().enter_liminal_state(
        &"did:dao:transitioning".to_string(),
        LiminalEntityType::Dao,
        Some("Restructuring governance model".to_string()),
    );

    // Tick updates entity count
    handler.on_tick(&state).unwrap();

    // Verify recategorization is blocked
    assert!(
        handler
            .engine()
            .is_recategorization_blocked(&"did:dao:transitioning".to_string()),
        "Recategorization should be blocked during liminal phase"
    );

    // Verify metrics
    let metrics = handler.collect_metrics();
    assert_eq!(metrics["entities_in_transition"].as_u64().unwrap(), 1);

    // Exit phase
    handler.on_exit(&state).unwrap();
}

// =========================================================================
// Test: Negative Capability Phase Operations
// =========================================================================

#[test]
fn test_negative_capability_holds_uncertain_claims() {
    let mut config = NegativeCapabilityConfig::default();
    config.max_hold_days = 3;

    let mut handler = NegativeCapabilityPhaseHandler::new(config);
    let state = make_state(1, CyclePhase::NegativeCapability, 0);

    // Enter phase
    handler.on_enter(&state).unwrap();

    // Hold a claim in uncertainty
    handler.engine_mut().hold_in_uncertainty(
        "complex-economic-claim",
        "Requires more research on long-term effects",
        3, // min hold days
        "did:agent:economist",
    );

    // Verify claim is held
    assert!(handler.engine().is_held("complex-economic-claim"));
    assert!(
        !handler.engine().can_vote_on("complex-economic-claim"),
        "Voting should be blocked on held claims"
    );

    // Tick should not release yet (within hold period)
    let events = handler.on_tick(&state).unwrap();
    let released_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, LivingProtocolEvent::ClaimReleasedFromUncertainty(_)))
        .collect();
    assert!(
        released_events.is_empty(),
        "Should not release claims within min hold period"
    );

    // Verify metrics
    let metrics = handler.collect_metrics();
    assert_eq!(metrics["claims_held"].as_u64().unwrap(), 1);

    // Exit phase
    handler.on_exit(&state).unwrap();
}

// =========================================================================
// Test: Eros Phase Operations
// =========================================================================

#[test]
fn test_eros_phase_computes_attractor_fields() {
    use living_core::KVectorSignature;
    use std::collections::HashMap;

    // Enable eros_attractor feature
    let mut features = FeatureFlags::default();
    features.eros_attractor = true;

    let mut handler = ErosPhaseHandler::new(features);
    let state = make_state(1, CyclePhase::Eros, 0);

    // Enter phase
    handler.on_enter(&state).unwrap();

    // Create K-vectors for attractor computation
    let now = Utc::now();
    let alice_k = KVectorSignature::from_array(
        [0.9, 0.8, 0.7, 0.6, 0.1, 0.2, 0.3, 0.9], // High in first dims, low in middle
        now,
    );
    let bob_k = KVectorSignature::from_array(
        [0.1, 0.2, 0.3, 0.4, 0.9, 0.8, 0.7, 0.85], // Complementary: low in first, high in middle
        now,
    );

    let mut k_vectors: HashMap<String, KVectorSignature> = HashMap::new();
    k_vectors.insert("did:agent:alice".to_string(), alice_k);
    k_vectors.insert("did:agent:bob".to_string(), bob_k);

    // Compute attractor fields
    let events = handler
        .engine_mut()
        .compute_attractor_fields(&k_vectors)
        .unwrap();

    // Should produce attractor field events for complementary agents
    assert!(
        !events.is_empty(),
        "Should compute attractor fields for complementary agents"
    );

    // Verify field strength is bounded
    for event in &events {
        assert!(
            event.field_strength >= 0.0 && event.field_strength <= 1.0,
            "Field strength should be in [0.0, 1.0]"
        );
    }

    // Tick (eros is on-demand, tick does nothing)
    handler.on_tick(&state).unwrap();

    // Exit phase
    handler.on_exit(&state).unwrap();
}

// =========================================================================
// Test: CoCreation Phase Operations
// =========================================================================

#[test]
fn test_cocreation_phase_forms_and_decays_entanglements() {
    let mut config = EntanglementConfig::default();
    config.min_co_creation_events = 1;
    config.decay_rate_per_day = 0.1;

    let mut handler = CoCreationPhaseHandler::new(config);
    let state = make_state(1, CyclePhase::CoCreation, 0);

    // Enter phase
    handler.on_enter(&state).unwrap();

    // Record co-creation event
    handler.engine_mut().record_co_creation(
        &"did:agent:alice".to_string(),
        &"did:agent:bob".to_string(),
        "Collaborated on governance proposal",
        0.9,
    );

    // Form entanglement
    let event = handler
        .engine_mut()
        .form_entanglement(&"did:agent:alice".to_string(), &"did:agent:bob".to_string())
        .unwrap();

    assert!(
        event.pair.entanglement_strength > 0.0,
        "Entanglement should have positive strength"
    );

    // Tick triggers decay
    let _events = handler.on_tick(&state).unwrap();

    // Exit phase
    handler.on_exit(&state).unwrap();

    // Verify metrics
    let metrics = handler.collect_metrics();
    assert!(metrics["phase"].as_str().unwrap() == "co_creation");
}

// =========================================================================
// Test: Beauty Phase Operations
// =========================================================================

#[test]
fn test_beauty_phase_scores_proposals() {
    let mut handler = BeautyPhaseHandler::new();
    let state = make_state(1, CyclePhase::Beauty, 0);

    // Enter phase
    handler.on_enter(&state).unwrap();

    // Score a proposal
    let event = handler.engine_mut().score_proposal(
        "proposal-elegant-solution",
        "A minimal implementation that achieves maximum impact through careful \
         composition of existing primitives, maintaining pattern consistency \
         while introducing novel cross-cutting concerns elegantly.",
        "did:scorer:beauty-validator",
        &[
            "existing-pattern-1".to_string(),
            "existing-pattern-2".to_string(),
        ],
        &["requirement-1".to_string(), "requirement-2".to_string()],
    );

    assert!(
        event.score.composite > 0.0 && event.score.composite <= 1.0,
        "Beauty score should be in [0.0, 1.0]"
    );

    // Tick updates scored count
    handler.on_tick(&state).unwrap();

    // Exit phase
    handler.on_exit(&state).unwrap();

    // Verify metrics
    let metrics = handler.collect_metrics();
    assert_eq!(metrics["proposals_scored"].as_u64().unwrap(), 1);
}

// =========================================================================
// Test: Emergent Personhood Phase Operations
// =========================================================================

#[test]
fn test_emergent_personhood_phase_runs() {
    let mut handler = EmergentPersonhoodPhaseHandler::new();
    let state = make_state(1, CyclePhase::EmergentPersonhood, 0);

    // Enter phase
    handler.on_enter(&state).unwrap();

    // Tick (Phi computation needs external K-vectors)
    handler.on_tick(&state).unwrap();

    // Exit phase
    handler.on_exit(&state).unwrap();

    // Verify metrics
    let metrics = handler.collect_metrics();
    assert_eq!(metrics["phase"].as_str().unwrap(), "emergent_personhood");
}

// =========================================================================
// Test: Kenosis Phase Operations
// =========================================================================

#[test]
fn test_kenosis_phase_handles_voluntary_release() {
    let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
    let mut handler = KenosisPhaseHandler::new(KenosisConfig::default(), event_bus);

    let state = make_state(1, CyclePhase::Kenosis, 0);

    // Enter phase (sets cycle number on engine)
    handler.on_enter(&state).unwrap();

    // Register agents
    handler
        .engine_mut()
        .register_agent("did:agent:generous", 100.0);
    handler
        .engine_mut()
        .register_agent("did:agent:strategic", 200.0);

    // Commit kenosis
    let commitment1 = handler
        .engine_mut()
        .commit_kenosis("did:agent:generous", 0.15) // 15%
        .unwrap();

    let commitment2 = handler
        .engine_mut()
        .commit_kenosis("did:agent:strategic", 0.20) // Max 20%
        .unwrap();

    // Verify commitments
    assert_eq!(commitment1.cycle_number, 1);
    assert!(commitment1.irrevocable);
    assert_eq!(commitment1.reputation_released, 15.0); // 15% of 100

    assert_eq!(commitment2.reputation_released, 40.0); // 20% of 200

    // Execute kenosis
    let (before1, after1) = handler
        .engine_mut()
        .execute_kenosis(&commitment1.id)
        .unwrap();
    assert_eq!(before1, 100.0);
    assert_eq!(after1, 85.0);

    // Verify 20% cap per cycle is enforced
    // Agent already committed 15%, so trying another 10% should cap at remaining 5%
    let result = handler
        .engine_mut()
        .commit_kenosis("did:agent:generous", 0.10);
    // This should succeed but be capped to 5% (20% - 15% already committed)
    assert!(
        result.is_ok(),
        "Should succeed but cap at remaining allowance"
    );
    let capped_commitment = result.unwrap();
    assert!(
        (capped_commitment.release_percentage - 0.05).abs() < f64::EPSILON,
        "Should be capped to remaining 5%"
    );

    // Now trying to commit more should fail (already at 20%)
    let third_result = handler
        .engine_mut()
        .commit_kenosis("did:agent:generous", 0.01);
    assert!(
        third_result.is_err(),
        "Should fail: already at 20% cap for this cycle"
    );

    // Tick (kenosis is event-driven)
    handler.on_tick(&state).unwrap();

    // Exit phase
    handler.on_exit(&state).unwrap();

    // Verify metrics
    let metrics = handler.collect_metrics();
    assert_eq!(metrics["phase"].as_str().unwrap(), "kenosis");
}

// =========================================================================
// Test: Cross-Primitive Interactions
// =========================================================================

#[test]
fn test_shadow_surfacing_to_composting_flow() {
    // Simulate the flow: shadow surfaces content -> composting processes it

    // 1. Shadow phase surfaces suppressed content
    let mut shadow_handler = ShadowPhaseHandler::new(0.3, ShadowConfig::default());

    shadow_handler.engine_mut().record_suppression(
        "failed-proposal-xyz",
        "Did not reach quorum",
        0.7,
        0.3,
        false,
    );

    let shadow_state = make_state(1, CyclePhase::Shadow, 1);
    let shadow_events = shadow_handler.on_tick(&shadow_state).unwrap();

    // Verify content was surfaced
    assert!(
        !shadow_events.is_empty(),
        "Shadow should surface the suppressed content"
    );

    // 2. Extract the surfaced content ID
    let surfaced_content_id = if let LivingProtocolEvent::ShadowSurfaced(event) = &shadow_events[0]
    {
        event.shadow.original_content_id.clone()
    } else {
        panic!("Expected ShadowSurfaced event");
    };

    // 3. Composting phase processes the surfaced content
    let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
    let mut composting_handler =
        CompostingPhaseHandler::new(living_core::CompostingConfig::default(), event_bus);

    let composting_state = make_state(1, CyclePhase::Composting, 0);
    composting_handler.on_enter(&composting_state).unwrap();

    // Start composting the surfaced content
    let record = composting_handler
        .engine_mut()
        .start_composting(
            CompostableEntity::FailedProposal,
            surfaced_content_id.clone(),
            CompostingReason::ProposalFailed {
                vote_count: 8,
                required: 20,
            },
        )
        .unwrap();

    // Extract learnings from the failed content
    composting_handler
        .engine_mut()
        .extract_nutrient(
            &record.id,
            "Proposal needed better community engagement before formal submission".to_string(),
            test_classification(),
        )
        .unwrap();

    // Complete composting
    let nutrients = composting_handler
        .engine_mut()
        .complete_composting(&record.id)
        .unwrap();

    assert_eq!(nutrients.len(), 1);
    assert_eq!(nutrients[0].source_entity, surfaced_content_id);
}

#[test]
fn test_entanglement_influences_co_creation() {
    let mut config = EntanglementConfig::default();
    config.min_co_creation_events = 2;

    let mut handler = CoCreationPhaseHandler::new(config);
    let state = make_state(1, CyclePhase::CoCreation, 0);

    handler.on_enter(&state).unwrap();

    // First co-creation event
    handler.engine_mut().record_co_creation(
        &"did:agent:researcher1".to_string(),
        &"did:agent:researcher2".to_string(),
        "Co-authored research paper",
        0.85,
    );

    // Not enough events yet for entanglement
    let result = handler.engine_mut().form_entanglement(
        &"did:agent:researcher1".to_string(),
        &"did:agent:researcher2".to_string(),
    );
    assert!(
        result.is_err(),
        "Should not form entanglement with insufficient co-creation"
    );

    // Second co-creation event
    handler.engine_mut().record_co_creation(
        &"did:agent:researcher1".to_string(),
        &"did:agent:researcher2".to_string(),
        "Joint presentation at conference",
        0.9,
    );

    // Now entanglement can form
    let event = handler
        .engine_mut()
        .form_entanglement(
            &"did:agent:researcher1".to_string(),
            &"did:agent:researcher2".to_string(),
        )
        .unwrap();

    assert!(
        event.pair.entanglement_strength > 0.0,
        "Should form entanglement with positive strength"
    );
}

// =========================================================================
// Test: Multi-Cycle Simulation
// =========================================================================

#[test]
fn test_multi_cycle_simulation_with_state_persistence() {
    let mut engine = create_test_engine();
    engine.start().unwrap();

    // Run 3 complete cycles
    for expected_cycle in 1..=3 {
        assert_eq!(engine.cycle_number(), expected_cycle);

        // Progress through all 9 phases
        for phase_idx in 0..9 {
            let events = engine.force_transition().unwrap();

            // Each transition should emit events
            assert!(
                !events.is_empty(),
                "Cycle {} transition {} should emit events",
                expected_cycle,
                phase_idx
            );
        }
    }

    // After 3 cycles, should be in cycle 4
    assert_eq!(engine.cycle_number(), 4);
    assert_eq!(engine.current_phase(), CyclePhase::Shadow);

    // Verify full transition history
    assert_eq!(
        engine.transition_history().len(),
        27, // 9 transitions * 3 cycles
        "Should have recorded all transitions"
    );
}

// =========================================================================
// Test: Realistic Multi-Agent Scenario
// =========================================================================

#[test]
fn test_realistic_multi_agent_cycle() {
    // Simulate a network with multiple agents going through a full cycle

    // Define agents with varying reputations (normalized to [0.0, 1.0] for shadow integration)
    // We'll use both absolute rep for kenosis and normalized for shadow
    let agents = [
        ("did:agent:whale", 1000.0, 0.95),  // High-rep agent
        ("did:agent:dolphin", 500.0, 0.7),  // Medium-rep agent
        ("did:agent:minnow", 50.0, 0.15),   // Low-rep agent (< 0.3 threshold)
        ("did:agent:newcomer", 10.0, 0.05), // New agent
    ];

    // ===== SHADOW PHASE =====
    let mut shadow_handler = ShadowPhaseHandler::new(0.25, ShadowConfig::default());
    let shadow_state = make_state(1, CyclePhase::Shadow, 0);

    // Minnow's content was suppressed by whale
    // Use normalized reputation for shadow integration (0.0-1.0)
    shadow_handler.engine_mut().record_suppression(
        "minnow-proposal",
        "Whale disagreed with approach",
        agents[0].2, // whale's normalized rep (0.95)
        agents[2].2, // minnow's normalized rep (0.15 - below 0.3 threshold)
        false,
    );

    shadow_handler.on_enter(&shadow_state).unwrap();
    let shadow_events = shadow_handler.on_tick(&shadow_state).unwrap();
    shadow_handler.on_exit(&shadow_state).unwrap();

    // Low-rep dissent should be surfaced
    let surfaced_count = shadow_events
        .iter()
        .filter(|e| {
            if let LivingProtocolEvent::ShadowSurfaced(ev) = e {
                ev.shadow.low_rep_dissent
            } else {
                false
            }
        })
        .count();
    assert!(surfaced_count > 0, "Should surface low-rep dissent");

    // ===== COMPOSTING PHASE =====
    let event_bus: Arc<dyn EventBus> = Arc::new(InMemoryEventBus::new());
    let mut composting_handler =
        CompostingPhaseHandler::new(living_core::CompostingConfig::default(), event_bus.clone());
    let composting_state = make_state(1, CyclePhase::Composting, 0);

    composting_handler.on_enter(&composting_state).unwrap();

    // Compost a failed DAO governance proposal
    let record = composting_handler
        .engine_mut()
        .start_composting(
            CompostableEntity::FailedProposal,
            "dao-restructure-v1".to_string(),
            CompostingReason::ProposalFailed {
                vote_count: 15,
                required: 30,
            },
        )
        .unwrap();

    composting_handler
        .engine_mut()
        .extract_nutrient(
            &record.id,
            "Need more community discussion before major governance changes".to_string(),
            test_classification(),
        )
        .unwrap();

    composting_handler
        .engine_mut()
        .complete_composting(&record.id)
        .unwrap();
    composting_handler.on_tick(&composting_state).unwrap();
    composting_handler.on_exit(&composting_state).unwrap();

    // ===== NEGATIVE CAPABILITY PHASE =====
    let mut nc_config = NegativeCapabilityConfig::default();
    nc_config.max_hold_days = 7;
    let mut nc_handler = NegativeCapabilityPhaseHandler::new(nc_config);
    let nc_state = make_state(1, CyclePhase::NegativeCapability, 0);

    nc_handler.on_enter(&nc_state).unwrap();

    // Hold a complex claim in uncertainty
    nc_handler.engine_mut().hold_in_uncertainty(
        "protocol-sustainability-claim",
        "Long-term economic sustainability needs modeling",
        5, // min hold days
        "did:agent:economist",
    );

    nc_handler.on_tick(&nc_state).unwrap();
    assert!(nc_handler.engine().is_held("protocol-sustainability-claim"));
    nc_handler.on_exit(&nc_state).unwrap();

    // ===== CO-CREATION PHASE =====
    let mut entanglement_config = EntanglementConfig::default();
    entanglement_config.min_co_creation_events = 1;
    let mut cocreation_handler = CoCreationPhaseHandler::new(entanglement_config);
    let cocreation_state = make_state(1, CyclePhase::CoCreation, 0);

    cocreation_handler.on_enter(&cocreation_state).unwrap();

    // Dolphin and minnow collaborate (use .0 for DID)
    cocreation_handler.engine_mut().record_co_creation(
        &agents[1].0.to_string(), // dolphin DID
        &agents[2].0.to_string(), // minnow DID
        "Joint development of community tool",
        0.88,
    );

    cocreation_handler
        .engine_mut()
        .form_entanglement(&agents[1].0.to_string(), &agents[2].0.to_string())
        .unwrap();

    cocreation_handler.on_tick(&cocreation_state).unwrap();
    cocreation_handler.on_exit(&cocreation_state).unwrap();

    // ===== KENOSIS PHASE =====
    let mut kenosis_handler = KenosisPhaseHandler::new(KenosisConfig::default(), event_bus);
    let kenosis_state = make_state(1, CyclePhase::Kenosis, 0);

    kenosis_handler.on_enter(&kenosis_state).unwrap();

    // Register all agents (use absolute reputation for kenosis)
    for (did, rep, _) in &agents {
        kenosis_handler.engine_mut().register_agent(did, *rep);
    }

    // Whale commits max kenosis (20%)
    let whale_commitment = kenosis_handler
        .engine_mut()
        .commit_kenosis(agents[0].0, 0.20)
        .unwrap();

    assert_eq!(whale_commitment.reputation_released, 200.0); // 20% of 1000
    assert!(whale_commitment.irrevocable);

    // Execute the kenosis
    let (before, after) = kenosis_handler
        .engine_mut()
        .execute_kenosis(&whale_commitment.id)
        .unwrap();

    assert_eq!(before, 1000.0);
    assert_eq!(after, 800.0);

    kenosis_handler.on_tick(&kenosis_state).unwrap();
    kenosis_handler.on_exit(&kenosis_state).unwrap();

    // Verify final state
    assert_eq!(
        kenosis_handler
            .engine()
            .get_reputation(agents[0].0)
            .unwrap(),
        800.0,
        "Whale's reputation should be reduced after kenosis"
    );
}

// =========================================================================
// Test: Phase Duration Correctness
// =========================================================================

#[test]
fn test_phase_durations_sum_to_28_days() {
    let total: u32 = CyclePhase::all_phases()
        .iter()
        .map(|p| p.duration_days())
        .sum();

    assert_eq!(total, 28, "Total cycle duration should be 28 days");

    // Verify individual durations
    assert_eq!(CyclePhase::Shadow.duration_days(), 2);
    assert_eq!(CyclePhase::Composting.duration_days(), 5);
    assert_eq!(CyclePhase::Liminal.duration_days(), 3);
    assert_eq!(CyclePhase::NegativeCapability.duration_days(), 3);
    assert_eq!(CyclePhase::Eros.duration_days(), 4);
    assert_eq!(CyclePhase::CoCreation.duration_days(), 7);
    assert_eq!(CyclePhase::Beauty.duration_days(), 2);
    assert_eq!(CyclePhase::EmergentPersonhood.duration_days(), 1);
    assert_eq!(CyclePhase::Kenosis.duration_days(), 1);
}

// =========================================================================
// Test: Phase-Specific Operation Permissions
// =========================================================================

#[test]
fn test_operation_permissions_per_phase() {
    let mut engine = create_test_engine();
    engine.start().unwrap();

    // Shadow phase: Gate 2 warnings suspended
    assert!(!engine.is_operation_permitted("gate2_warning"));
    assert!(engine.is_operation_permitted("vote"));
    assert!(engine.is_operation_permitted("read"));

    // Advance to Negative Capability
    engine.force_transition().unwrap(); // Composting
    engine.force_transition().unwrap(); // Liminal
    engine.force_transition().unwrap(); // NegativeCapability

    assert_eq!(engine.current_phase(), CyclePhase::NegativeCapability);
    assert!(!engine.is_operation_permitted("vote"));
    assert!(engine.is_operation_permitted("read"));

    // Advance to Kenosis
    engine.force_transition().unwrap(); // Eros
    engine.force_transition().unwrap(); // CoCreation
    engine.force_transition().unwrap(); // Beauty
    engine.force_transition().unwrap(); // EmergentPersonhood
    engine.force_transition().unwrap(); // Kenosis

    assert_eq!(engine.current_phase(), CyclePhase::Kenosis);
    assert!(engine.is_operation_permitted("kenosis"));
    assert!(engine.is_operation_permitted("read"));
}

// =========================================================================
// Test: Event Accumulation Across Cycle
// =========================================================================

#[test]
fn test_event_accumulation_across_cycle() {
    let mut engine = create_test_engine();
    engine.start().unwrap();

    let mut total_events = 0;

    // Run through one complete cycle
    for _ in 0..9 {
        let transition_events = engine.force_transition().unwrap();
        total_events += transition_events.len();
    }

    // Should have accumulated events
    assert!(
        total_events >= 9,
        "Should have at least one event per transition"
    );

    // Verify cycle events are tracked
    let cycle_events = engine.cycle_events();
    assert!(
        !cycle_events.is_empty(),
        "Cycle should have accumulated events"
    );
}

// =========================================================================
// Test: Transition History Recording
// =========================================================================

#[test]
fn test_transition_history_detailed_recording() {
    let mut engine = create_test_engine();
    engine.start().unwrap();

    // Make a few transitions
    engine.force_transition().unwrap(); // Shadow -> Composting
    engine.force_transition().unwrap(); // Composting -> Liminal
    engine.force_transition().unwrap(); // Liminal -> NegativeCapability

    let history = engine.transition_history();
    assert_eq!(history.len(), 3);

    // Verify first transition
    assert_eq!(history[0].from, CyclePhase::Shadow);
    assert_eq!(history[0].to, CyclePhase::Composting);
    assert_eq!(history[0].cycle_number, 1);

    // Verify second transition
    assert_eq!(history[1].from, CyclePhase::Composting);
    assert_eq!(history[1].to, CyclePhase::Liminal);

    // Verify third transition
    assert_eq!(history[2].from, CyclePhase::Liminal);
    assert_eq!(history[2].to, CyclePhase::NegativeCapability);
}

// =========================================================================
// Test: Custom Configuration
// =========================================================================

#[test]
fn test_engine_with_custom_configuration() {
    let mut config = LivingProtocolConfig::default();
    config.shadow.spectral_k_anomaly_threshold = 0.5;
    config.kenosis.max_release_per_cycle = 0.15; // More conservative 15%
    config.composting.max_nutrients_per_entity = 5;

    let mut engine = create_engine_with_config(config);
    engine.start().unwrap();

    // Verify engine starts correctly with custom config
    assert_eq!(engine.current_phase(), CyclePhase::Shadow);
    assert!(engine.is_running());

    // Run through a full cycle
    for _ in 0..9 {
        engine.force_transition().unwrap();
    }

    assert_eq!(engine.cycle_number(), 2);
}
